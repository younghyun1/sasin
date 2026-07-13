//! Tree structure operations: rename (display name only), duplicate, create folder/request
//! in a parent, and sibling reorder. Slugs never change on rename, so file identity, open
//! tabs, and git history stay stable.

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::messages::{MoveDir, TreeMsg};
use crate::gui::state::Tab;
use crate::model::{
    Folder, HttpRequest, Node, NodePath, children_mut, find_node, find_node_mut, insert_node,
    sibling_slugs,
};
use crate::storage::layout::unique_slug;

impl App {
    /// Dispatch a grouped tree operation (the single `Message::Tree` arm lands here).
    pub(super) fn tree_op(&mut self, op: TreeMsg) -> Task<Message> {
        match op {
            TreeMsg::RenameStart(path) => {
                let name = find_node(&self.workspace.root, &path)
                    .map(|n| n.display_name().to_string())
                    .unwrap_or_default();
                self.renaming = Some((path, name));
                Task::none()
            }
            TreeMsg::RenameInput(text) => {
                if let Some((_, buf)) = &mut self.renaming {
                    *buf = text;
                }
                Task::none()
            }
            TreeMsg::RenameCancel => {
                self.renaming = None;
                Task::none()
            }
            TreeMsg::RenameCommit => self.rename_commit(),
            TreeMsg::Duplicate(path) => self.duplicate(path),
            TreeMsg::NewFolder(parent) => self.new_folder(parent),
            TreeMsg::NewRequestIn(parent) => self.new_request_in(parent),
            TreeMsg::Move(path, dir) => self.move_sibling(path, dir),
        }
    }

    fn rename_commit(&mut self) -> Task<Message> {
        let Some((path, name)) = self.renaming.take() else {
            return Task::none();
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            return Task::none();
        }
        match find_node_mut(&mut self.workspace.root, &path) {
            Some(Node::Folder(f)) => f.name = name.clone(),
            Some(Node::Http(r)) => r.name = name.clone(),
            Some(Node::Ws(w)) => w.name = name.clone(),
            None => return Task::none(),
        }
        for tab in self.tabs.iter_mut().filter(|t| t.path == path) {
            tab.name = name.clone();
        }
        self.save_task()
    }

    fn duplicate(&mut self, path: NodePath) -> Task<Message> {
        let Some(original) = find_node(&self.workspace.root, &path) else {
            return Task::none();
        };
        let mut copy = original.clone();
        let parent = &path[..path.len().saturating_sub(1)];
        let mut taken = sibling_slugs(&self.workspace.root, parent);
        let slug = unique_slug(copy.slug(), &mut taken);
        let name = format!("{} copy", copy.display_name());
        match &mut copy {
            Node::Folder(f) => {
                f.slug = slug;
                f.name = name;
            }
            Node::Http(r) => {
                r.slug = slug;
                r.name = name;
            }
            Node::Ws(w) => {
                w.slug = slug;
                w.name = name;
            }
        }
        // Insert right after the original so the copy lands where the eye is.
        let index = children_mut(&mut self.workspace.root, parent)
            .and_then(|c| c.iter().position(|n| n.slug() == path[path.len() - 1]))
            .map(|i| i + 1);
        insert_node(&mut self.workspace.root, parent, index, copy);
        self.save_task()
    }

    fn new_folder(&mut self, parent: NodePath) -> Task<Message> {
        let mut taken = sibling_slugs(&self.workspace.root, &parent);
        let slug = unique_slug("new-folder", &mut taken);
        let folder = Folder::new(slug.clone(), "New Folder");
        if !insert_node(
            &mut self.workspace.root,
            &parent,
            None,
            Node::Folder(folder),
        ) {
            return Task::none();
        }
        // Reveal the new folder: expand the parent chain and the folder itself.
        let mut path = parent;
        path.push(slug);
        for len in 1..=path.len() {
            self.expanded.insert(path[..len].to_vec());
        }
        self.save_task()
    }

    /// Create a request under `parent` (empty = root), open it in a tab, and persist.
    pub(super) fn new_request_in(&mut self, parent: NodePath) -> Task<Message> {
        let mut taken = sibling_slugs(&self.workspace.root, &parent);
        let slug = unique_slug("new-request", &mut taken);
        let req = HttpRequest::new(slug.clone(), "New Request", "GET", "https://");
        if !insert_node(&mut self.workspace.root, &parent, None, Node::Http(req)) {
            return Task::none();
        }
        for len in 1..=parent.len() {
            self.expanded.insert(parent[..len].to_vec());
        }
        let mut path = parent;
        path.push(slug);
        if let Some(node) = find_node(&self.workspace.root, &path) {
            self.tabs.push(Tab::from_node(path, node));
            self.active = Some(self.tabs.len() - 1);
        }
        self.save_task()
    }

    /// Swap the node with its previous/next sibling. Persisted automatically: save
    /// regenerates the parent's `order` list from live child order.
    fn move_sibling(&mut self, path: NodePath, dir: MoveDir) -> Task<Message> {
        let parent = &path[..path.len().saturating_sub(1)];
        let Some(children) = children_mut(&mut self.workspace.root, parent) else {
            return Task::none();
        };
        let Some(idx) = children
            .iter()
            .position(|n| n.slug() == path[path.len() - 1])
        else {
            return Task::none();
        };
        let target = match dir {
            MoveDir::Up if idx > 0 => idx - 1,
            MoveDir::Down if idx + 1 < children.len() => idx + 1,
            _ => return Task::none(),
        };
        children.swap(idx, target);
        self.save_task()
    }
}
