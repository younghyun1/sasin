//! Workspace bootstrapping: load an existing workspace, migrate a legacy dataset, or init a default.

use std::path::Path;

use crate::model::{Folder, HttpRequest, Node, Workspace};
use crate::storage::{load_workspace, migrate_legacy, save_workspace};

/// Resolve the workspace at startup. Returns the workspace and an optional status note.
pub(super) fn load_or_init(dir: &Path, legacy: &Path) -> (Workspace, Option<String>) {
    if dir.join("sasin.toml").exists() {
        return match load_workspace(dir) {
            Ok(ws) => (ws, None),
            Err(e) => (default_workspace(), Some(format!("Load failed: {e}"))),
        };
    }
    if legacy.exists() {
        return match migrate_legacy(legacy, dir) {
            Ok(ws) => (
                ws,
                Some("Migrated legacy dataset into a workspace.".to_string()),
            ),
            Err(e) => init_default(dir, Some(format!("Migration failed: {e}"))),
        };
    }
    init_default(dir, None)
}

fn init_default(dir: &Path, note: Option<String>) -> (Workspace, Option<String>) {
    let ws = default_workspace();
    let status = match save_workspace(dir, &ws) {
        Ok(()) => note,
        Err(e) => Some(format!("Could not create workspace: {e}")),
    };
    (ws, status)
}

fn default_workspace() -> Workspace {
    let mut ws = Workspace::default_with_name("My Workspace");
    let req = HttpRequest::new("example", "Example", "GET", "https://example.com");
    ws.root = vec![Node::Folder(Folder {
        children: vec![Node::Http(req)],
        ..Folder::new("my-collection", "My Collection")
    })];
    ws
}
