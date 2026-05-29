//! GUI session state: open tabs and the glue between editor buffers and the workspace model.

use iced::widget::text_editor;

use crate::model::{Body, KvEntry, Node, NodePath, RawLang};
use crate::models::{HttpMethod, ResponseModel};

/// What a tab is editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Http,
    Ws,
}

/// An open editor tab bound to a node by its path. Editor buffers live here; the workspace tree
/// is updated from them on save (and read into them on open).
#[derive(Debug)]
pub struct Tab {
    pub path: NodePath,
    pub kind: TabKind,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    /// Raw `Name: Value` lines (one structured KV table replaces this in P3).
    pub headers_text: String,
    /// Set once the user edits the raw headers. Gates header write-back so a request's
    /// disabled headers — which the raw-text view cannot represent — are not silently dropped
    /// when the user only edits the url/body. The full KV table (P3) removes this limitation.
    pub headers_edited: bool,
    pub body: text_editor::Content,
    pub dirty: bool,
    pub sending: bool,
    /// Generation of the in-flight send, used to drop stale results.
    pub send_gen: u64,
    pub response: Option<ResponseModel>,
    pub error: Option<String>,
}

impl Tab {
    /// Build a tab from a node (HTTP or WebSocket). Folders are not openable; callers must guard.
    pub fn from_node(path: NodePath, node: &Node) -> Self {
        let base = |kind, name: String, method, url, headers_text, body_text: String| Tab {
            path: path.clone(),
            kind,
            name,
            method,
            url,
            headers_text,
            headers_edited: false,
            body: text_editor::Content::with_text(&body_text),
            dirty: false,
            sending: false,
            send_gen: 0,
            response: None,
            error: None,
        };
        match node {
            Node::Http(r) => base(
                TabKind::Http,
                display_name(&r.name, &r.slug),
                HttpMethod::parse(&r.method).unwrap_or_default(),
                r.url.clone(),
                headers_to_text(&r.headers),
                raw_body_text(&r.body),
            ),
            Node::Ws(w) => base(
                TabKind::Ws,
                display_name(&w.name, &w.slug),
                HttpMethod::Get,
                w.url.clone(),
                headers_to_text(&w.headers),
                String::new(),
            ),
            Node::Folder(f) => base(
                TabKind::Http,
                display_name(&f.name, &f.slug),
                HttpMethod::Get,
                String::new(),
                String::new(),
                String::new(),
            ),
        }
    }
}

fn display_name(name: &str, slug: &str) -> String {
    if name.is_empty() {
        slug.to_string()
    } else {
        name.to_string()
    }
}

fn raw_body_text(body: &Body) -> String {
    match body {
        Body::Raw { text, .. } => text.clone(),
        _ => String::new(),
    }
}

/// Write a tab's editor buffers back into its node. Returns an error if edited headers don't
/// parse. Headers are only rewritten when the user actually edited them (see [`Tab::headers_edited`]),
/// so requests with disabled headers keep them when only the url/body changed.
pub fn apply_tab_to_node(tab: &Tab, node: &mut Node) -> Result<(), String> {
    let kvs: Option<Vec<KvEntry>> = if tab.headers_edited {
        Some(
            parse_header_lines(&tab.headers_text)?
                .into_iter()
                .map(|(key, value)| KvEntry {
                    key,
                    value,
                    enabled: true,
                })
                .collect(),
        )
    } else {
        None
    };

    match node {
        Node::Http(r) => {
            r.method = tab.method.as_str().to_string();
            r.url = tab.url.clone();
            if let Some(kvs) = kvs {
                r.headers = kvs;
            }
            let body_text = tab.body.text();
            r.body = if body_text.trim().is_empty() {
                Body::None
            } else {
                let language = match &r.body {
                    Body::Raw { language, .. } => *language,
                    _ => RawLang::default(),
                };
                Body::Raw {
                    language,
                    text: body_text,
                }
            };
        }
        Node::Ws(w) => {
            w.url = tab.url.clone();
            if let Some(kvs) = kvs {
                w.headers = kvs;
            }
        }
        Node::Folder(_) => {}
    }
    Ok(())
}

/// Parse `Name: Value` lines. Blank lines and `#` comments are ignored.
pub fn parse_header_lines(raw: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(format!(
                "Invalid header on line {} (expected `Name: Value`): {line}",
                i + 1
            ));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("Empty header name on line {}", i + 1));
        }
        out.push((name.to_string(), value.trim().to_string()));
    }
    Ok(out)
}

/// Format enabled headers as `Name: Value` lines for the raw editor.
pub fn headers_to_text(headers: &[KvEntry]) -> String {
    let mut s = String::new();
    for h in headers {
        if !h.enabled || h.key.trim().is_empty() {
            continue;
        }
        s.push_str(h.key.trim());
        s.push_str(": ");
        s.push_str(h.value.trim());
        s.push('\n');
    }
    s
}
