//! GUI session state: open tabs.
//!
//! Structured request fields (method, url, params, headers, auth, body mode, settings) are edited
//! directly on the workspace node via the update handlers. Only the free-form body text needs a
//! buffered [`text_editor::Content`], held here and synced into the node on edit.

use iced::widget::text_editor;

use crate::gui::messages::{EditorPanel, KvOp};
use crate::model::{Body, FormKind, FormPart, KvEntry, Node, NodePath, Variable};
use crate::models::ResponseModel;

/// What a tab is editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Http,
    Ws,
}

/// An open editor tab bound to a node by its path.
#[derive(Debug)]
pub struct Tab {
    pub path: NodePath,
    pub kind: TabKind,
    pub name: String,
    pub panel: EditorPanel,
    /// Editable buffer for the numeric request timeout (the model stores `u64`).
    pub timeout_text: String,
    /// Primary body text buffer (raw body, or the GraphQL query).
    pub body: text_editor::Content,
    /// GraphQL variables buffer.
    pub gql_vars: text_editor::Content,
    pub dirty: bool,
    pub sending: bool,
    /// Generation of the in-flight send, used to drop stale results.
    pub send_gen: u64,
    pub response: Option<ResponseModel>,
    pub error: Option<String>,
}

impl Tab {
    /// Build a tab from a node, seeding the body buffers from its stored body.
    pub fn from_node(path: NodePath, node: &Node) -> Self {
        let (kind, name, body, vars) = match node {
            Node::Http(r) => (
                TabKind::Http,
                display_name(&r.name, &r.slug),
                primary_body_text(&r.body),
                gql_vars_text(&r.body),
            ),
            Node::Ws(w) => (
                TabKind::Ws,
                display_name(&w.name, &w.slug),
                String::new(),
                String::new(),
            ),
            Node::Folder(f) => (
                TabKind::Http,
                display_name(&f.name, &f.slug),
                String::new(),
                String::new(),
            ),
        };
        let timeout_text = match node {
            Node::Http(r) => r.settings.timeout_ms.to_string(),
            _ => "30000".to_string(),
        };
        Self {
            path,
            kind,
            name,
            panel: EditorPanel::Params,
            timeout_text,
            body: text_editor::Content::with_text(&body),
            gql_vars: text_editor::Content::with_text(&vars),
            dirty: false,
            sending: false,
            send_gen: 0,
            response: None,
            error: None,
        }
    }

    /// Reseed the body buffers from `body` (after a body-mode switch).
    pub fn reload_body(&mut self, body: &Body) {
        self.body = text_editor::Content::with_text(&primary_body_text(body));
        self.gql_vars = text_editor::Content::with_text(&gql_vars_text(body));
    }
}

/// Write the body text buffers back into the node (called on body edits and before send/save).
pub fn sync_body(tab: &Tab, node: &mut Node) {
    if let Node::Http(r) = node {
        match &mut r.body {
            Body::Raw { text, .. } => *text = tab.body.text(),
            Body::GraphQl { query, variables } => {
                *query = tab.body.text();
                *variables = tab.gql_vars.text();
            }
            _ => {}
        }
    }
}

fn primary_body_text(body: &Body) -> String {
    match body {
        Body::Raw { text, .. } => text.clone(),
        Body::GraphQl { query, .. } => query.clone(),
        _ => String::new(),
    }
}

fn gql_vars_text(body: &Body) -> String {
    match body {
        Body::GraphQl { variables, .. } => variables.clone(),
        _ => String::new(),
    }
}

fn display_name(name: &str, slug: &str) -> String {
    if name.is_empty() {
        slug.to_string()
    } else {
        name.to_string()
    }
}

/// Apply a row op to a key/value list (params, headers, url-encoded fields).
pub(crate) fn apply_kv_entry(list: &mut Vec<KvEntry>, op: KvOp) {
    match op {
        KvOp::Add => list.push(KvEntry {
            key: String::new(),
            value: String::new(),
            enabled: true,
        }),
        KvOp::Remove(i) => {
            if i < list.len() {
                list.remove(i);
            }
        }
        KvOp::Key(i, s) => {
            if let Some(e) = list.get_mut(i) {
                e.key = s;
            }
        }
        KvOp::Value(i, s) => {
            if let Some(e) = list.get_mut(i) {
                e.value = s;
            }
        }
        KvOp::Toggle(i, b) => {
            if let Some(e) = list.get_mut(i) {
                e.enabled = b;
            }
        }
    }
}

/// Apply a row op to an environment's variables (preserving secret/description).
pub(crate) fn apply_kv_variable(vars: &mut Vec<Variable>, op: KvOp) {
    match op {
        KvOp::Add => vars.push(Variable::new(String::new(), String::new())),
        KvOp::Remove(i) => {
            if i < vars.len() {
                vars.remove(i);
            }
        }
        KvOp::Key(i, s) => {
            if let Some(v) = vars.get_mut(i) {
                v.key = s;
            }
        }
        KvOp::Value(i, s) => {
            if let Some(v) = vars.get_mut(i) {
                v.value = s;
            }
        }
        KvOp::Toggle(i, b) => {
            if let Some(v) = vars.get_mut(i) {
                v.enabled = b;
            }
        }
    }
}

/// Apply a row op to a form-data parts list.
pub(crate) fn apply_kv_formpart(parts: &mut Vec<FormPart>, op: KvOp) {
    match op {
        KvOp::Add => parts.push(FormPart {
            key: String::new(),
            kind: FormKind::Text,
            value: String::new(),
            src: String::new(),
            enabled: true,
        }),
        KvOp::Remove(i) => {
            if i < parts.len() {
                parts.remove(i);
            }
        }
        KvOp::Key(i, s) => {
            if let Some(p) = parts.get_mut(i) {
                p.key = s;
            }
        }
        KvOp::Value(i, s) => {
            if let Some(p) = parts.get_mut(i) {
                p.value = s;
            }
        }
        KvOp::Toggle(i, b) => {
            if let Some(p) = parts.get_mut(i) {
                p.enabled = b;
            }
        }
    }
}
