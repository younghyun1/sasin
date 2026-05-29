//! The workspace: in-memory root of the tree plus the on-disk `sasin.toml` manifest.

use serde::{Deserialize, Serialize};

use crate::model::defaults::{default_timeout_ms, default_true};
use crate::model::environment::{Environment, Variable};
use crate::model::tree::Node;

/// Transport defaults inherited by every request unless overridden (`[defaults]` in `sasin.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDefaults {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_true")]
    pub verify_tls: bool,
    #[serde(default = "default_true")]
    pub use_cookie_jar: bool,
}

impl Default for WorkspaceDefaults {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
            verify_tls: true,
            use_cookie_jar: true,
        }
    }
}

/// In-memory workspace. The tree (`root`), `environments`, and `globals` come from individual
/// files; only `name`/`defaults`/top-level order persist in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub name: String,
    pub defaults: WorkspaceDefaults,
    pub root: Vec<Node>,
    pub environments: Vec<Environment>,
    pub globals: Vec<Variable>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::default_with_name("My Workspace")
    }
}

impl Workspace {
    /// An empty workspace with the given name and default settings.
    pub fn default_with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            defaults: WorkspaceDefaults::default(),
            root: Vec::new(),
            environments: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Build the on-disk manifest, deriving top-level child order from the current tree.
    pub fn manifest(&self) -> WorkspaceManifest {
        WorkspaceManifest {
            name: self.name.clone(),
            order: self.root.iter().map(|n| n.slug().to_string()).collect(),
            defaults: self.defaults.clone(),
        }
    }
}

/// On-disk `sasin.toml`. `order` (an inline array) precedes the `[defaults]` table, as TOML
/// requires values before tables.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    #[serde(default)]
    pub defaults: WorkspaceDefaults,
}
