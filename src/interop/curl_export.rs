//! Render an [`HttpRequest`] as a copy-pasteable `curl` command.

use crate::model::{ApiKeyLoc, Auth, Body, FormKind, HttpRequest};

/// Build a multi-line curl command for `req` (literal values; variables are not interpolated).
pub fn to_curl(req: &HttpRequest) -> String {
    let mut parts: Vec<String> = vec!["curl".to_string()];

    if !req.method.eq_ignore_ascii_case("GET") {
        parts.push(format!("-X {}", req.method));
    }
    // Reuse the send path's URL builder so params are percent-encoded exactly as sent.
    let url =
        crate::http::exec::effective_url(&req.url, &req.params).unwrap_or_else(|_| req.url.clone());
    parts.push(format!("'{}'", quote(&url)));

    for h in &req.headers {
        if h.enabled && !h.key.trim().is_empty() {
            parts.push(format!("-H '{}: {}'", quote(&h.key), quote(&h.value)));
        }
    }

    match &req.auth {
        Auth::Basic { user, pass } => parts.push(format!("-u '{}:{}'", quote(user), quote(pass))),
        Auth::Bearer { token } | Auth::OAuth2 { token } => {
            parts.push(format!("-H 'Authorization: Bearer {}'", quote(token)))
        }
        Auth::ApiKey {
            key,
            value,
            add_to: ApiKeyLoc::Header,
        } => parts.push(format!("-H '{}: {}'", quote(key), quote(value))),
        _ => {}
    }

    match &req.body {
        Body::Raw { text, .. } if !text.is_empty() => parts.push(format!("-d '{}'", quote(text))),
        Body::UrlEncoded { fields } => {
            for f in fields {
                if f.enabled {
                    parts.push(format!(
                        "--data-urlencode '{}={}'",
                        quote(&f.key),
                        quote(&f.value)
                    ));
                }
            }
        }
        Body::FormData { parts: form } => {
            for p in form {
                if p.enabled {
                    let v = if matches!(p.kind, FormKind::File) {
                        format!("@{}", p.src)
                    } else {
                        p.value.clone()
                    };
                    parts.push(format!("-F '{}={}'", quote(&p.key), quote(&v)));
                }
            }
        }
        Body::Binary { file } => parts.push(format!("--data-binary '@{}'", quote(file))),
        Body::GraphQl { query, variables } => {
            let vars: serde_json::Value = serde_json::from_str(variables)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let payload = serde_json::json!({ "query": query, "variables": vars });
            parts.push("-H 'Content-Type: application/json'".to_string());
            parts.push(format!("-d '{}'", quote(&payload.to_string())));
        }
        _ => {}
    }

    parts.join(" \\\n  ")
}

/// Escape single quotes for inclusion inside a single-quoted shell string: `'` becomes `'\''`.
fn quote(s: &str) -> String {
    s.replace('\'', "'\\''")
}
