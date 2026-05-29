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

/// A path to a node: the sequence of slugs from a root to the node.
pub type NodePath = Vec<String>;

/// Find a node by its slug path, walking folders. `None` if any segment is missing.
pub fn find_node<'a>(roots: &'a [Node], path: &[String]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let node = roots.iter().find(|n| n.slug() == first)?;
    if rest.is_empty() {
        Some(node)
    } else if let Node::Folder(folder) = node {
        find_node(&folder.children, rest)
    } else {
        None
    }
}

/// Mutable variant of [`find_node`].
pub fn find_node_mut<'a>(roots: &'a mut [Node], path: &[String]) -> Option<&'a mut Node> {
    let (first, rest) = path.split_first()?;
    let node = roots.iter_mut().find(|n| n.slug() == first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        match node {
            Node::Folder(folder) => find_node_mut(&mut folder.children, rest),
            _ => None,
        }
    }
}

/// Resolve the effective auth for the node at `path`: the node's own auth if it is not
/// [`Auth::Inherit`], otherwise the nearest ancestor folder's auth, otherwise [`Auth::None`].
pub fn resolve_auth(roots: &[Node], path: &[String]) -> Auth {
    if let Some(node) = find_node(roots, path) {
        let own = match node {
            Node::Folder(f) => &f.auth,
            Node::Http(r) => &r.auth,
            Node::Ws(w) => &w.auth,
        };
        if !own.is_inherit() {
            return own.clone();
        }
    }
    // Walk ancestor folders, deepest first.
    for len in (1..path.len()).rev() {
        if let Some(Node::Folder(f)) = find_node(roots, &path[..len])
            && !f.auth.is_inherit()
        {
            return f.auth.clone();
        }
    }
    Auth::None
}

/// Remove and return the node at `path`, if present.
pub fn remove_node(roots: &mut Vec<Node>, path: &[String]) -> Option<Node> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        let idx = roots.iter().position(|n| n.slug() == first)?;
        Some(roots.remove(idx))
    } else {
        match roots.iter_mut().find(|n| n.slug() == first)? {
            Node::Folder(folder) => remove_node(&mut folder.children, rest),
            _ => None,
        }
    }
}
