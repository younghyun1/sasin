//! Workspace bootstrapping: load an existing workspace, migrate a legacy dataset, or init a default.

use std::path::Path;

use crate::gui::app::App;
use crate::gui::state::Tab;
use crate::model::{Folder, HttpRequest, Node, NodePath, Workspace, find_node};
use crate::storage::{load_workspace, migrate_legacy, save_workspace};

impl App {
    /// Reload the workspace from disk after an external change (e.g. a git pull or branch switch).
    ///
    /// The reload is applied only when the on-disk tree actually differs from what is in memory,
    /// so our own saves — which write exactly what is in memory — never trigger a reload loop.
    /// Tabs whose node has disappeared are closed; surviving tabs keep their buffers.
    pub(super) fn reload_workspace(&mut self) {
        let loaded = match load_workspace(&self.workspace_dir) {
            Ok(ws) => ws,
            Err(_) => return,
        };
        if loaded == self.workspace {
            return;
        }
        self.workspace = loaded;

        let active_path = self
            .active
            .and_then(|a| self.tabs.get(a))
            .map(|t| t.path.clone());
        let root = &self.workspace.root;
        self.tabs
            .retain(|t| matches!(find_node(root, &t.path), Some(Node::Http(_) | Node::Ws(_))));
        self.active = match active_path.and_then(|p| self.tabs.iter().position(|t| t.path == p)) {
            Some(i) => Some(i),
            None if self.tabs.is_empty() => None,
            None => Some(0),
        };
        if self
            .active_env
            .is_some_and(|i| i >= self.workspace.environments.len())
        {
            self.active_env = if self.workspace.environments.is_empty() {
                None
            } else {
                Some(0)
            };
        }
        // Drop websocket sessions whose node disappeared.
        let root = &self.workspace.root;
        self.ws
            .retain(|rt| matches!(find_node(root, &rt.path), Some(Node::Ws(_))));

        // Reseed non-dirty tabs from the reloaded nodes so their editor buffers reflect the new
        // on-disk content. Without this the next send/save would flush the *stale* buffers back
        // into the node via sync_body/sync_scripts, silently clobbering the external change.
        // Tabs with unsaved edits (dirty) are left alone to preserve the user's work.
        let to_reseed: Vec<(usize, NodePath)> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.dirty)
            .map(|(i, t)| (i, t.path.clone()))
            .collect();
        for (i, path) in to_reseed {
            if let Some(node) = find_node(&self.workspace.root, &path) {
                let panel = self.tabs[i].panel;
                let mut fresh = Tab::from_node(path, node);
                fresh.panel = panel;
                self.tabs[i] = fresh;
            }
        }
        let dirty_kept = self.tabs.iter().filter(|t| t.dirty).count();

        // A run in progress may reference paths that changed; close the panel to be safe.
        self.runner = None;
        self.status = Some(if dirty_kept > 0 {
            format!(
                "Workspace reloaded from disk ({dirty_kept} tab(s) with unsaved edits kept — they may differ from disk)."
            )
        } else {
            "Workspace reloaded from disk.".to_string()
        });
    }
}

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
