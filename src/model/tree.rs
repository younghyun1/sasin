//! The collection tree: folders containing nested folders, HTTP requests, and websocket requests.
//!
//! [`Node`] is an in-memory construct only — each node maps to its own file on disk, so the
//! tree is never serialized as one blob. [`Folder`] *is* serialized, but only its metadata
//! (`folder.toml`); `slug` comes from the directory name and `children` come from the filesystem.

use serde::{Deserialize, Serialize};

use crate::model::auth::Auth;
use crate::model::environment::Variable;
use crate::model::request::HttpRequest;
use crate::model::scripts::Scripts;
use crate::model::websocket::WsRequest;

/// One entry in the collection tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Folder(Folder),
    Http(HttpRequest),
    Ws(WsRequest),
}

impl Node {
    /// File-stem identity of this node within its parent directory.
    pub fn slug(&self) -> &str {
        match self {
            Node::Folder(f) => &f.slug,
            Node::Http(r) => &r.slug,
            Node::Ws(w) => &w.slug,
        }
    }

    /// Stored display name (may be empty).
    pub fn name(&self) -> &str {
        match self {
            Node::Folder(f) => &f.name,
            Node::Http(r) => &r.name,
            Node::Ws(w) => &w.name,
        }
    }

    /// Display name, falling back to the slug when the stored name is empty.
    pub fn display_name(&self) -> &str {
        let name = self.name();
        if name.is_empty() { self.slug() } else { name }
    }
}

/// Folder metadata (`folder.toml`): display name, ordering of children, and inherited
/// auth/variables/scripts. `slug` and `children` are filesystem-derived and not serialized.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Folder {
    #[serde(skip)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Explicit child ordering (slugs, no extension). Missing entries fall back to lexical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    #[serde(default, skip_serializing_if = "Auth::is_inherit")]
    pub auth: Auth,
    #[serde(default, rename = "variable", skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
    #[serde(default, skip_serializing_if = "Scripts::is_empty")]
    pub scripts: Scripts,
    #[serde(skip)]
    pub children: Vec<Node>,
}

impl Folder {
    /// Create an empty folder with the given slug and display name.
    pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            name: name.into(),
            ..Self::default()
        }
    }
}
