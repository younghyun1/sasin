//! Variable interpolation: resolve `{{name}}` and dynamic `{{$timestamp}}`-style tokens across a
//! request before it is sent. The stored request is never mutated; a resolved clone is produced.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Auth, Body, Environment, HttpRequest, Variable};

/// Resolved variable scope: globals overlaid by the active environment (and, later, script-set
/// locals). Last writer wins, so the environment overrides globals.
#[derive(Debug, Default, Clone)]
pub struct VarContext {
    map: HashMap<String, String>,
}

impl VarContext {
    /// Build from globals overlaid by an optional active environment (enabled variables only).
    pub fn from_scopes(globals: &[Variable], environment: Option<&Environment>) -> Self {
        let mut map = HashMap::new();
        for v in globals {
            if v.enabled {
                map.insert(v.key.clone(), v.value.clone());
            }
        }
        if let Some(env) = environment {
            for v in &env.variables {
                if v.enabled {
                    map.insert(v.key.clone(), v.value.clone());
                }
            }
        }
        Self { map }
    }

    /// Set a variable (used by pre-request scripts).
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.map.insert(key.into(), value.into());
    }

    /// Overlay a scope of variables (enabled only); later overlays win. Used to layer folder
    /// (collection) variables between globals and the environment, Postman-style.
    pub fn overlay_variables(&mut self, vars: &[Variable]) {
        for v in vars {
            if v.enabled {
                self.map.insert(v.key.clone(), v.value.clone());
            }
        }
    }

    /// A snapshot of the current variables (for injection into a script engine).
    pub fn snapshot(&self) -> HashMap<String, String> {
        self.map.clone()
    }

    /// Look up a name: dynamic generator if it starts with `$`, otherwise a scope variable.
    fn lookup(&self, name: &str) -> Option<String> {
        if let Some(stripped) = name.strip_prefix('$') {
            dynamic(stripped)
        } else {
            self.map.get(name).cloned()
        }
    }
}

/// Replace `{{name}}` tokens in `input`. Unknown names are left literal (so a typo is visible
/// rather than silently blanked).
pub fn interpolate(input: &str, ctx: &VarContext) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let name = after[..end].trim();
                match ctx.lookup(name) {
                    Some(value) => out.push_str(&value),
                    None => {
                        out.push_str("{{");
                        out.push_str(&after[..end]);
                        out.push_str("}}");
                    }
                }
                rest = &after[end + 2..];
            }
            None => {
                // No closing braces; emit the remainder verbatim.
                out.push_str("{{");
                out.push_str(after);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Produce a resolved clone of `req` with every interpolatable field expanded.
pub fn resolve_request(req: &HttpRequest, ctx: &VarContext) -> HttpRequest {
    let mut r = req.clone();
    r.url = interpolate(&r.url, ctx);
    for p in &mut r.params {
        p.key = interpolate(&p.key, ctx);
        p.value = interpolate(&p.value, ctx);
    }
    for h in &mut r.headers {
        h.key = interpolate(&h.key, ctx);
        h.value = interpolate(&h.value, ctx);
    }
    r.auth = resolve_auth(&r.auth, ctx);
    r.body = resolve_body(&r.body, ctx);
    r
}

fn resolve_auth(auth: &Auth, ctx: &VarContext) -> Auth {
    match auth {
        Auth::None => Auth::None,
        Auth::Inherit => Auth::Inherit,
        Auth::Basic { user, pass } => Auth::Basic {
            user: interpolate(user, ctx),
            pass: interpolate(pass, ctx),
        },
        Auth::Bearer { token } => Auth::Bearer {
            token: interpolate(token, ctx),
        },
        Auth::OAuth2 { token } => Auth::OAuth2 {
            token: interpolate(token, ctx),
        },
        Auth::ApiKey { key, value, add_to } => Auth::ApiKey {
            key: interpolate(key, ctx),
            value: interpolate(value, ctx),
            add_to: *add_to,
        },
    }
}

fn resolve_body(body: &Body, ctx: &VarContext) -> Body {
    match body {
        Body::None => Body::None,
        Body::Raw { language, text } => Body::Raw {
            language: *language,
            text: interpolate(text, ctx),
        },
        Body::UrlEncoded { fields } => Body::UrlEncoded {
            fields: fields
                .iter()
                .map(|f| crate::model::KvEntry {
                    key: interpolate(&f.key, ctx),
                    value: interpolate(&f.value, ctx),
                    enabled: f.enabled,
                })
                .collect(),
        },
        Body::FormData { parts } => Body::FormData {
            parts: parts
                .iter()
                .map(|p| crate::model::FormPart {
                    key: interpolate(&p.key, ctx),
                    kind: p.kind,
                    value: interpolate(&p.value, ctx),
                    src: interpolate(&p.src, ctx),
                    enabled: p.enabled,
                })
                .collect(),
        },
        Body::Binary { file } => Body::Binary {
            file: interpolate(file, ctx),
        },
        Body::GraphQl { query, variables } => Body::GraphQl {
            query: interpolate(query, ctx),
            variables: interpolate(variables, ctx),
        },
    }
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

/// Dynamic variable generators (`name` is given without the leading `$`).
fn dynamic(name: &str) -> Option<String> {
    match name {
        "timestamp" => Some((now_nanos() / 1_000_000_000).to_string()),
        "isoTimestamp" => Some(chrono::Utc::now().to_rfc3339()),
        "randomInt" => Some(((now_nanos() % 1000) as u64).to_string()),
        "randomUUID" | "guid" => Some(pseudo_uuid()),
        _ => None,
    }
}

/// A unique, v4-shaped identifier. Not cryptographically random — derived from time + a process
/// counter — but unique enough for request testing.
fn pseudo_uuid() -> String {
    let n = now_nanos() as u64;
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let a = n ^ c.rotate_left(17);
    let b = n.rotate_left(32) ^ c.rotate_left(40);
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16,
        (a as u16) & 0x0fff,
        ((b >> 48) as u16 & 0x3fff) | 0x8000,
        b & 0xffff_ffff_ffff,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Variable;

    fn ctx() -> VarContext {
        VarContext::from_scopes(&[Variable::new("g", "gv")], None)
    }

    #[test]
    fn interpolates_known_leaves_unknown() {
        let c = VarContext::from_scopes(&[Variable::new("base", "https://h")], None);
        assert_eq!(interpolate("{{base}}/x", &c), "https://h/x");
        assert_eq!(interpolate("{{missing}}/x", &c), "{{missing}}/x");
        assert_eq!(interpolate("no vars", &c), "no vars");
        assert_eq!(interpolate("{{ base }}", &c), "https://h");
    }

    #[test]
    fn env_overrides_globals() {
        let env = Environment {
            slug: "dev".into(),
            name: "dev".into(),
            variables: vec![Variable::new("g", "env-v")],
        };
        let c = VarContext::from_scopes(&[Variable::new("g", "global-v")], Some(&env));
        assert_eq!(interpolate("{{g}}", &c), "env-v");
    }

    #[test]
    fn dynamic_timestamp_and_uuid() {
        let c = ctx();
        let ts = interpolate("{{$timestamp}}", &c);
        assert!(ts.parse::<u64>().is_ok(), "timestamp not numeric: {ts}");
        let id = interpolate("{{$randomUUID}}", &c);
        assert_eq!(id.len(), 36, "uuid wrong length: {id}");
        assert_eq!(id.as_bytes()[14], b'4', "version nibble");
    }

    #[test]
    fn resolves_request_fields() {
        let c = VarContext::from_scopes(
            &[
                Variable::new("h", "example.com"),
                Variable::new("tok", "abc"),
            ],
            None,
        );
        let mut req = HttpRequest::new("r", "R", "GET", "https://{{h}}/p");
        req.headers.push(crate::model::KvEntry::new("X", "{{tok}}"));
        req.auth = Auth::Bearer {
            token: "{{tok}}".into(),
        };
        let resolved = resolve_request(&req, &c);
        assert_eq!(resolved.url, "https://example.com/p");
        assert_eq!(resolved.headers[0].value, "abc");
        assert!(matches!(resolved.auth, Auth::Bearer { token } if token == "abc"));
    }
}
