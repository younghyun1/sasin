//! Top-level application: owns the workspace + open tabs and routes messages.
//!
//! Split across submodules to keep files focused: [`boot`] (workspace load/init), [`commands`]
//! (async send/save), and [`view`] (rendering). They are child modules of `app`, so they may use
//! `App`'s private fields.

mod boot;
mod commands;
mod view;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::Task;

use crate::gui::Message;
use crate::gui::messages::SplitId;
use crate::gui::state::{self, Tab};
use crate::http::HttpClientConfig;
use crate::model::{HttpRequest, Node, NodePath, Workspace, find_node, find_node_mut, remove_node};
use crate::persist::{app_state_dir, default_dataset_path};
use crate::storage::delete_node;
use crate::storage::layout::unique_slug;

use boot::load_or_init;

/// The application state.
pub struct App {
    workspace_dir: PathBuf,
    workspace: Workspace,
    expanded: HashSet<NodePath>,
    tabs: Vec<Tab>,
    active: Option<usize>,
    http_config: HttpClientConfig,
    send_gen: u64,
    active_abort: Option<tokio::task::AbortHandle>,
    pretty_json: bool,
    show_headers: bool,
    sidebar_px: f32,
    editor_px: f32,
    status: Option<String>,
}

impl App {
    /// Boot: locate the workspace directory, loading/migrating/initializing as needed.
    pub fn new() -> (Self, Task<Message>) {
        let dir = app_state_dir().join("workspace");
        let legacy = default_dataset_path();
        let (workspace, status) = load_or_init(&dir, &legacy);
        let app = Self {
            workspace_dir: dir,
            workspace,
            expanded: HashSet::new(),
            tabs: Vec::new(),
            active: None,
            http_config: HttpClientConfig::default(),
            send_gen: 0,
            active_abort: None,
            pretty_json: true,
            show_headers: true,
            sidebar_px: 300.0,
            editor_px: 360.0,
            status,
        };
        (app, Task::none())
    }

    /// Window title (bound via `Application::title`).
    pub fn title(&self) -> String {
        let name = if self.workspace.name.is_empty() {
            "workspace"
        } else {
            &self.workspace.name
        };
        format!("sasin — {name}")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleFolder(path) => {
                if !self.expanded.remove(&path) {
                    self.expanded.insert(path);
                }
                Task::none()
            }
            Message::OpenNode(path) => {
                if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
                    self.active = Some(i);
                } else if let Some(node) = find_node(&self.workspace.root, &path)
                    && !matches!(node, Node::Folder(_))
                {
                    self.tabs.push(Tab::from_node(path, node));
                    self.active = Some(self.tabs.len() - 1);
                }
                Task::none()
            }
            Message::NewRequest => {
                let mut taken: HashSet<String> = self
                    .workspace
                    .root
                    .iter()
                    .map(|n| n.slug().to_string())
                    .collect();
                let slug = unique_slug("new-request", &mut taken);
                let req = HttpRequest::new(slug.clone(), "New Request", "GET", "https://");
                self.workspace.root.push(Node::Http(req));
                let path = vec![slug];
                if let Some(node) = find_node(&self.workspace.root, &path) {
                    self.tabs.push(Tab::from_node(path, node));
                    self.active = Some(self.tabs.len() - 1);
                }
                self.save_task()
            }
            Message::DeleteNode(path) => {
                // Keep the active selection pinned to its tab across the removal.
                let active_path = self
                    .active
                    .and_then(|a| self.tabs.get(a))
                    .map(|t| t.path.clone());
                self.tabs.retain(|t| !t.path.starts_with(&path));
                remove_node(&mut self.workspace.root, &path);
                if let Err(e) = delete_node(&self.workspace_dir, &path) {
                    self.status = Some(format!("Delete failed: {e}"));
                }
                self.active = active_path.and_then(|p| self.tabs.iter().position(|t| t.path == p));
                self.save_task()
            }
            Message::SelectTab(i) => {
                if i < self.tabs.len() {
                    self.active = Some(i);
                }
                Task::none()
            }
            Message::CloseTab(i) => {
                if i >= self.tabs.len() {
                    return Task::none();
                }
                let mut task = Task::none();
                if self.tabs[i].dirty {
                    let name = self.tabs[i].name.clone();
                    let path = self.tabs[i].path.clone();
                    let applied = match find_node_mut(&mut self.workspace.root, &path) {
                        Some(node) => state::apply_tab_to_node(&self.tabs[i], node),
                        None => Ok(()),
                    };
                    match applied {
                        Ok(()) => task = self.save_task(),
                        Err(e) => {
                            self.status =
                                Some(format!("Closed \"{name}\" with unsaved changes: {e}"))
                        }
                    }
                }
                self.tabs.remove(i);
                // Preserve the selected tab's identity across the index shift.
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
                task
            }
            Message::MethodChanged(method) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.method = method;
                    tab.dirty = true;
                }
                Task::none()
            }
            Message::UrlChanged(url) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.url = url;
                    tab.dirty = true;
                }
                Task::none()
            }
            Message::HeadersChanged(s) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.headers_text = s;
                    tab.headers_edited = true;
                    tab.dirty = true;
                }
                Task::none()
            }
            Message::BodyAction(action) => {
                if let Some(tab) = self.active_tab_mut() {
                    let edited = action.is_edit();
                    tab.body.perform(action);
                    if edited {
                        tab.dirty = true;
                    }
                }
                Task::none()
            }
            Message::SaveActiveTab => {
                let Some(i) = self.active else {
                    return Task::none();
                };
                let path = self.tabs[i].path.clone();
                if let Some(node) = find_node_mut(&mut self.workspace.root, &path) {
                    match state::apply_tab_to_node(&self.tabs[i], node) {
                        Ok(()) => {
                            self.tabs[i].dirty = false;
                            self.tabs[i].error = None;
                        }
                        Err(e) => {
                            self.tabs[i].error = Some(e);
                            return Task::none();
                        }
                    }
                }
                self.save_task()
            }
            Message::Saved(result) => {
                self.status = match result {
                    Ok(()) => None,
                    Err(e) => Some(format!("Save failed: {e}")),
                };
                Task::none()
            }
            Message::SendPressed => self.send_active(),
            Message::CancelPressed => {
                if let Some(abort) = self.active_abort.take() {
                    abort.abort();
                }
                if let Some(tab) = self.active_tab_mut() {
                    tab.sending = false;
                    tab.error = Some("Cancelled".to_string());
                }
                Task::none()
            }
            Message::RequestFinished(send_id, resp) => {
                if let Some(tab) = self
                    .tabs
                    .iter_mut()
                    .find(|t| t.send_gen == send_id && t.sending)
                {
                    tab.sending = false;
                    tab.response = Some(resp);
                    tab.error = None;
                }
                self.active_abort = None;
                Task::none()
            }
            Message::RequestFailed(send_id, err) => {
                if let Some(tab) = self
                    .tabs
                    .iter_mut()
                    .find(|t| t.send_gen == send_id && t.sending)
                {
                    tab.sending = false;
                    tab.response = None;
                    tab.error = Some(err);
                }
                self.active_abort = None;
                Task::none()
            }
            Message::TogglePrettyJson => {
                self.pretty_json = !self.pretty_json;
                Task::none()
            }
            Message::ToggleShowHeaders => {
                self.show_headers = !self.show_headers;
                Task::none()
            }
            Message::SplitDragged(id, px) => {
                match id {
                    SplitId::Sidebar => self.sidebar_px = px.clamp(220.0, 560.0),
                    SplitId::RequestResponse => self.editor_px = px.clamp(220.0, 900.0),
                }
                Task::none()
            }
        }
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }
}
