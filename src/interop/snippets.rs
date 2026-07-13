//! Code-snippet generation for a request: curl (delegated), HTTPie, JS fetch, Python requests.
//! Literal values; `{{variables}}` are not interpolated, matching copy-as-curl.

use std::fmt;

use crate::interop::to_curl;
use crate::model::{ApiKeyLoc, Auth, Body, FormKind, HttpRequest};

/// Snippet target language/tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SnippetLang {
    #[default]
    Curl,
    Httpie,
    JsFetch,
    PythonRequests,
}

impl SnippetLang {
    pub const fn all() -> &'static [SnippetLang] {
        &[
            SnippetLang::Curl,
            SnippetLang::Httpie,
            SnippetLang::JsFetch,
            SnippetLang::PythonRequests,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            SnippetLang::Curl => "curl",
            SnippetLang::Httpie => "HTTPie",
            SnippetLang::JsFetch => "JS fetch",
            SnippetLang::PythonRequests => "Python requests",
        }
    }
}

impl fmt::Display for SnippetLang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Render `req` as a snippet in `lang`.
pub fn to_snippet(req: &HttpRequest, lang: SnippetLang) -> String {
    match lang {
        SnippetLang::Curl => to_curl(req),
        SnippetLang::Httpie => to_httpie(req),
        SnippetLang::JsFetch => to_fetch(req),
        SnippetLang::PythonRequests => to_python(req),
    }
}

/// URL with enabled params merged, exactly as the send path encodes them.
fn full_url(req: &HttpRequest) -> String {
    crate::http::exec::effective_url(&req.url, &req.params).unwrap_or_else(|_| req.url.clone())
}

/// Enabled headers plus the auth header when auth renders as one.
fn effective_headers(req: &HttpRequest) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    match &req.auth {
        Auth::Bearer { token } | Auth::OAuth2 { token } => {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        Auth::ApiKey {
            key,
            value,
            add_to: ApiKeyLoc::Header,
        } => headers.push((key.clone(), value.clone())),
        _ => {}
    }
    headers
}

fn shell_quote(s: &str) -> String {
    s.replace('\'', "'\\''")
}

fn to_httpie(req: &HttpRequest) -> String {
    let mut parts: Vec<String> = vec!["http".to_string()];
    if let Auth::Basic { user, pass } = &req.auth {
        parts.push(format!("-a '{}:{}'", shell_quote(user), shell_quote(pass)));
    }
    if matches!(&req.body, Body::UrlEncoded { .. } | Body::FormData { .. }) {
        parts.push("--form".to_string());
    }
    parts.push(req.method.to_ascii_uppercase());
    parts.push(format!("'{}'", shell_quote(&full_url(req))));
    for (k, v) in effective_headers(req) {
        parts.push(format!("'{}:{}'", shell_quote(&k), shell_quote(&v)));
    }
    match &req.body {
        Body::Raw { text, .. } if !text.is_empty() => {
            parts.push(format!("--raw '{}'", shell_quote(text)));
        }
        Body::UrlEncoded { fields } => {
            for f in fields.iter().filter(|f| f.enabled) {
                parts.push(format!(
                    "'{}={}'",
                    shell_quote(&f.key),
                    shell_quote(&f.value)
                ));
            }
        }
        Body::FormData { parts: form } => {
            for p in form.iter().filter(|p| p.enabled) {
                if matches!(p.kind, FormKind::File) {
                    parts.push(format!("'{}@{}'", shell_quote(&p.key), shell_quote(&p.src)));
                } else {
                    parts.push(format!(
                        "'{}={}'",
                        shell_quote(&p.key),
                        shell_quote(&p.value)
                    ));
                }
            }
        }
        Body::Binary { file } => parts.push(format!("< '{}'", shell_quote(file))),
        Body::GraphQl { query, variables } => {
            parts.push(format!(
                "--raw '{}'",
                shell_quote(&graphql_payload(query, variables))
            ));
        }
        _ => {}
    }
    parts.join(" \\\n  ")
}

fn to_fetch(req: &HttpRequest) -> String {
    let url = json_str(&full_url(req));
    let mut opts: Vec<String> = vec![format!("  method: {}", json_str(&req.method))];
    let headers = effective_headers(req);
    if !headers.is_empty() {
        let list = headers
            .iter()
            .map(|(k, v)| format!("    {}: {}", json_str(k), json_str(v)))
            .collect::<Vec<_>>()
            .join(",\n");
        opts.push(format!("  headers: {{\n{list}\n  }}"));
    }
    match &req.body {
        Body::Raw { text, .. } if !text.is_empty() => {
            opts.push(format!("  body: {}", json_str(text)));
        }
        Body::UrlEncoded { fields } => {
            let list = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| format!("    {}: {}", json_str(&f.key), json_str(&f.value)))
                .collect::<Vec<_>>()
                .join(",\n");
            opts.push(format!("  body: new URLSearchParams({{\n{list}\n  }})"));
        }
        Body::FormData { parts } => {
            let mut lines = vec!["const form = new FormData();".to_string()];
            for p in parts.iter().filter(|p| p.enabled) {
                if matches!(p.kind, FormKind::File) {
                    lines.push(format!(
                        "form.append({}, /* file: {} */ fileInput.files[0]);",
                        json_str(&p.key),
                        p.src
                    ));
                } else {
                    lines.push(format!(
                        "form.append({}, {});",
                        json_str(&p.key),
                        json_str(&p.value)
                    ));
                }
            }
            opts.push("  body: form".to_string());
            return format!(
                "{}\nconst res = await fetch({url}, {{\n{}\n}});\nconsole.log(res.status, await res.text());",
                lines.join("\n"),
                opts.join(",\n"),
            );
        }
        Body::Binary { file } => {
            opts.push(format!("  body: /* bytes of {file} */ fileBytes"));
        }
        Body::GraphQl { query, variables } => {
            opts.push(format!(
                "  body: {}",
                json_str(&graphql_payload(query, variables))
            ));
        }
        _ => {}
    }
    format!(
        "const res = await fetch({url}, {{\n{}\n}});\nconsole.log(res.status, await res.text());",
        opts.join(",\n")
    )
}

fn to_python(req: &HttpRequest) -> String {
    let method = req.method.to_ascii_lowercase();
    let known = matches!(
        method.as_str(),
        "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
    );
    let mut args: Vec<String> = vec![format!("    {}", json_str(&full_url(req)))];
    if !known {
        args.insert(
            0,
            format!("    {}", json_str(&req.method.to_ascii_uppercase())),
        );
    }
    let headers = effective_headers(req);
    if !headers.is_empty() {
        let list = headers
            .iter()
            .map(|(k, v)| format!("        {}: {}", json_str(k), json_str(v)))
            .collect::<Vec<_>>()
            .join(",\n");
        args.push(format!("    headers={{\n{list}\n    }}"));
    }
    if let Auth::Basic { user, pass } = &req.auth {
        args.push(format!("    auth=({}, {})", json_str(user), json_str(pass)));
    }
    match &req.body {
        Body::Raw { text, .. } if !text.is_empty() => {
            args.push(format!("    data={}", json_str(text)));
        }
        Body::UrlEncoded { fields } => {
            let list = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| format!("        {}: {}", json_str(&f.key), json_str(&f.value)))
                .collect::<Vec<_>>()
                .join(",\n");
            args.push(format!("    data={{\n{list}\n    }}"));
        }
        Body::FormData { parts } => {
            let mut data = Vec::new();
            let mut files = Vec::new();
            for p in parts.iter().filter(|p| p.enabled) {
                if matches!(p.kind, FormKind::File) {
                    files.push(format!(
                        "        {}: open({}, \"rb\")",
                        json_str(&p.key),
                        json_str(&p.src)
                    ));
                } else {
                    data.push(format!(
                        "        {}: {}",
                        json_str(&p.key),
                        json_str(&p.value)
                    ));
                }
            }
            if !data.is_empty() {
                args.push(format!("    data={{\n{}\n    }}", data.join(",\n")));
            }
            if !files.is_empty() {
                args.push(format!("    files={{\n{}\n    }}", files.join(",\n")));
            }
        }
        Body::Binary { file } => {
            args.push(format!("    data=open({}, \"rb\")", json_str(file)));
        }
        Body::GraphQl { query, variables } => {
            args.push(format!(
                "    data={}",
                json_str(&graphql_payload(query, variables))
            ));
        }
        _ => {}
    }
    let call = if known {
        format!("requests.{method}(")
    } else {
        "requests.request(".to_string()
    };
    format!(
        "import requests\n\nres = {call}\n{},\n)\nprint(res.status_code, res.text)",
        args.join(",\n")
    )
}

/// JSON `{query, variables}` envelope shared by the graphql arms (mirrors the send path).
fn graphql_payload(query: &str, variables: &str) -> String {
    let vars: serde_json::Value = serde_json::from_str(variables)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    serde_json::json!({ "query": query, "variables": vars }).to_string()
}

/// A JSON string literal (used for JS and Python quoting; both accept JSON escapes).
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KvEntry;

    fn sample() -> HttpRequest {
        let mut req = HttpRequest::new("r", "R", "POST", "https://api.dev/things");
        req.headers.push(KvEntry {
            key: "Accept".into(),
            value: "application/json".into(),
            enabled: true,
        });
        req.auth = Auth::Bearer {
            token: "tok".into(),
        };
        req.body = Body::Raw {
            language: crate::model::RawLang::Json,
            text: "{\"a\":1}".into(),
        };
        req
    }

    #[test]
    fn httpie_renders_method_url_headers_body() {
        let s = to_httpie(&sample());
        assert!(s.starts_with("http"));
        assert!(s.contains("POST"));
        assert!(s.contains("'https://api.dev/things'"));
        assert!(s.contains("'Authorization:Bearer tok'"));
        assert!(s.contains("--raw"));
    }

    #[test]
    fn fetch_renders_options_object() {
        let s = to_fetch(&sample());
        assert!(s.contains("await fetch(\"https://api.dev/things\""));
        assert!(s.contains("method: \"POST\""));
        assert!(s.contains("\"Authorization\": \"Bearer tok\""));
        assert!(s.contains("body: \"{\\\"a\\\":1}\""));
    }

    #[test]
    fn python_uses_verb_helper_and_auth() {
        let mut req = sample();
        req.auth = Auth::Basic {
            user: "u".into(),
            pass: "p".into(),
        };
        let s = to_python(&req);
        assert!(s.contains("requests.post("));
        assert!(s.contains("auth=(\"u\", \"p\")"));
        assert!(s.contains("print(res.status_code"));
    }

    #[test]
    fn python_custom_verb_falls_back_to_request() {
        let mut req = sample();
        req.method = "PURGE".into();
        let s = to_python(&req);
        assert!(s.contains("requests.request("));
        assert!(s.contains("\"PURGE\""));
    }
}
