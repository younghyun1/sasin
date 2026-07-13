//! Tree/tab navigation mutations: deleting a node, opening history, and closing a tab.
//! (Creating/renaming/duplicating/moving nodes lives in [`super::tree_ops`].)

use std::collections::HashSet;

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state::Tab;
use crate::model::{HttpRequest, Node, NodePath, find_node, remove_node};
use crate::storage::layout::unique_slug;
use crate::storage::{HistoryRecord, delete_node};

impl App {
    /// Remove the node (and its subtree) at `path`, closing affected tabs, then persist.
    pub(super) fn delete_path(&mut self, path: NodePath) -> Task<Message> {
        // Keep the active selection pinned to its tab across the removal.
        let active_path = self
            .active
            .and_then(|a| self.tabs.get(a))
            .map(|t| t.path.clone());
        self.tabs.retain(|t| !t.path.starts_with(&path));
        // Tear down any websocket sessions under the removed subtree.
        self.ws.retain(|rt| !rt.path.starts_with(&path));
        remove_node(&mut self.workspace.root, &path);
        if let Err(e) = delete_node(&self.workspace_dir, &path) {
            self.status = Some(format!("Delete failed: {e}"));
        }
        self.active = active_path.and_then(|p| self.tabs.iter().position(|t| t.path == p));
        self.save_task()
    }

    /// Re-create a request from a history record (method + url) at the root and open it.
    pub(super) fn open_history(&mut self, record: HistoryRecord) -> Task<Message> {
        let mut taken: HashSet<String> = self
            .workspace
            .root
            .iter()
            .map(|n| n.slug().to_string())
            .collect();
        let slug = unique_slug("history", &mut taken);
        let req = HttpRequest::new(slug.clone(), &record.url, &record.method, &record.url);
        self.workspace.root.push(Node::Http(req));
        let path = vec![slug];
        if let Some(node) = find_node(&self.workspace.root, &path) {
            self.tabs.push(Tab::from_node(path, node));
            self.active = Some(self.tabs.len() - 1);
        }
        self.save_task()
    }

    /// Close the tab at index `i`, preserving the selected tab's identity across the index shift.
    pub(super) fn close_tab(&mut self, i: usize) -> Task<Message> {
        if i >= self.tabs.len() {
            return Task::none();
        }
        let dirty = self.tabs[i].dirty;
        self.tabs.remove(i);
        self.active = if self.tabs.is_empty() {
            None
        } else if let Some(a) = self.active {
            if a > i {
                Some(a - 1)
            } else {
                Some(a.min(self.tabs.len() - 1))
            }
        } else {
            None
        };
        if dirty {
            self.save_task()
        } else {
            Task::none()
        }
    }
}
