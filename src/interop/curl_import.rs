//! Parse a `curl` command line into an [`HttpRequest`].
//!
//! Supports the common flags: `-X/--request`, `-H/--header`, `-d/--data*`, `--data-urlencode`,
//! `-F/--form`, `-u/--user`, `-A/--user-agent`, `-b/--cookie`, `-e/--referer`, `-G/--get`,
//! `-L/--location`, `-k/--insecure`, `--url`, and a positional URL. Unknown flags are skipped.

use crate::model::{Auth, Body, FormKind, FormPart, HttpRequest, KvEntry, RawLang, Settings};

/// Parse a curl command. Returns an error if no URL is present.
pub fn from_curl(input: &str) -> Result<HttpRequest, String> {
    let tokens = tokenize(input);
    let mut it = tokens.iter().peekable();
    if it.peek().map(|s| s.as_str()) == Some("curl") {
        it.next();
    }

    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers: Vec<KvEntry> = Vec::new();
    let mut data: Vec<String> = Vec::new();
    let mut urlencoded: Vec<KvEntry> = Vec::new();
    let mut form: Vec<FormPart> = Vec::new();
    let mut auth = Auth::None;
    let mut get_with_data = false;
    let mut settings = Settings::default();

    while let Some(tok) = it.next() {
        match tok.as_str() {
            "-X" | "--request" => {
                if let Some(m) = it.next() {
                    method = Some(m.to_ascii_uppercase());
                }
            }
            "-H" | "--header" => {
                if let Some(h) = it.next()
                    && let Some((k, v)) = h.split_once(':')
                {
                    headers.push(KvEntry::new(k.trim(), v.trim()));
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-ascii" | "--data-binary" => {
                if let Some(d) = it.next() {
                    data.push(d.clone());
                }
            }
            "--data-urlencode" => {
                if let Some(d) = it.next() {
                    match d.split_once('=') {
                        Some((k, v)) => urlencoded.push(KvEntry::new(k, v)),
                        None => urlencoded.push(KvEntry::new(d.as_str(), "")),
                    }
                }
            }
            "-F" | "--form" => {
                if let Some(f) = it.next() {
                    form.push(parse_form(f));
                }
            }
            "-u" | "--user" => {
                if let Some(u) = it.next() {
                    let (user, pass) = u.split_once(':').unwrap_or((u.as_str(), ""));
                    auth = Auth::Basic {
                        user: user.to_string(),
                        pass: pass.to_string(),
                    };
                }
            }
            "-A" | "--user-agent" => {
                if let Some(a) = it.next() {
                    headers.push(KvEntry::new("User-Agent", a.as_str()));
                }
            }
            "-b" | "--cookie" => {
                if let Some(c) = it.next() {
                    headers.push(KvEntry::new("Cookie", c.as_str()));
                }
            }
            "-e" | "--referer" => {
                if let Some(r) = it.next() {
                    headers.push(KvEntry::new("Referer", r.as_str()));
                }
            }
            "-G" | "--get" => get_with_data = true,
            "-L" | "--location" => settings.follow_redirects = true,
            "-k" | "--insecure" => settings.verify_tls = false,
            "--url" => {
                if let Some(u) = it.next() {
                    url = Some(u.clone());
                }
            }
            other if other.starts_with('-') => {} // unknown flag: skip
            positional => {
                if url.is_none() {
                    url = Some(positional.to_string());
                }
            }
        }
    }

    let url = url.ok_or("no URL found in curl command")?;

    let has_data = !data.is_empty() || !urlencoded.is_empty() || !form.is_empty();
    let method = method.unwrap_or_else(|| {
        if has_data && !get_with_data {
            "POST".to_string()
        } else {
            "GET".to_string()
        }
    });

    let mut params: Vec<KvEntry> = Vec::new();
    let body = if !form.is_empty() {
        Body::FormData { parts: form }
    } else if !urlencoded.is_empty() {
        Body::UrlEncoded { fields: urlencoded }
    } else if !data.is_empty() {
        let joined = data.join("&");
        if get_with_data {
            for pair in joined.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    params.push(KvEntry::new(k, v));
                }
            }
            Body::None
        } else {
            let language = if serde_json::from_str::<serde_json::Value>(&joined).is_ok() {
                RawLang::Json
            } else {
                RawLang::Text
            };
            Body::Raw {
                language,
                text: joined,
            }
        }
    } else {
        Body::None
    };

    Ok(HttpRequest {
        method,
        url,
        params,
        headers,
        auth,
        body,
        settings,
        ..HttpRequest::new("imported", "Imported", "GET", "")
    })
}

fn parse_form(field: &str) -> FormPart {
    match field.split_once('=') {
        Some((k, v)) => match v.strip_prefix('@') {
            Some(file) => FormPart {
                key: k.to_string(),
                kind: FormKind::File,
                value: String::new(),
                src: file.to_string(),
                enabled: true,
            },
            None => FormPart {
                key: k.to_string(),
                kind: FormKind::Text,
                value: v.to_string(),
                src: String::new(),
                enabled: true,
            },
        },
        None => FormPart {
            key: field.to_string(),
            kind: FormKind::Text,
            value: String::new(),
            src: String::new(),
            enabled: true,
        },
    }
}

/// Shell-like tokenizer: handles single/double quotes, backslash escapes, and `\`-newline
/// line continuations (curl commands are often pasted multi-line).
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut has = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => match chars.next() {
                Some('\n') => {}
                Some(other) => {
                    cur.push(other);
                    has = true;
                }
                None => {}
            },
            '\'' if !in_double => {
                in_single = !in_single;
                has = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                has = true;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if has {
                    tokens.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        tokens.push(cur);
    }
    tokens
}
