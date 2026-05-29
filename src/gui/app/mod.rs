//! Top-level application: owns the workspace + open tabs.
//!
//! Behavior is split across child modules (they may use `App`'s private fields): [`boot`] loads the
//! workspace, [`update`] dispatches messages, [`edit`]/[`edit_body`] mutate the active node,
//! [`commands`] runs async send/save, [`ws`] drives websocket sessions, and [`view`] renders.

mod boot;
mod commands;
mod edit;
mod edit_body;
mod nav;
mod runner;
mod update;
mod view;
mod ws;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::Task;

use crate::gui::Message;
use crate::gui::runner_state::RunnerState;
use crate::gui::state::{Tab, WsRuntime};
use crate::http::HttpClientConfig;
use crate::model::{NodePath, Workspace};
use crate::persist::{app_state_dir, default_dataset_path};
use crate::storage::{HistoryCache, read_history};

use boot::load_or_init;

/// The application state.
pub struct App {
    workspace_dir: PathBuf,
    workspace: Workspace,
    expanded: HashSet<NodePath>,
    tabs: Vec<Tab>,
    active: Option<usize>,
    /// Index into `workspace.environments`; `None` means no active environment.
    active_env: Option<usize>,
    /// Buffer for the "import from curl" box.
    curl_import_text: String,
    /// The single active websocket session, if any.
    ws: Option<WsRuntime>,
    /// The active collection-runner session, if any.
    runner: Option<RunnerState>,
    http_config: HttpClientConfig,
    send_gen: u64,
    active_abort: Option<tokio::task::AbortHandle>,
    pretty_json: bool,
    response_tab: crate::gui::messages::ResponseTab,
    response_search: String,
    sidebar_px: f32,
    editor_px: f32,
    /// Persisted, recently-sent requests (newest last); shown in the sidebar.
    history: HistoryCache,
    status: Option<String>,
}

impl App {
    /// Boot: locate the workspace directory, loading/migrating/initializing as needed.
    pub fn new() -> (Self, Task<Message>) {
        let dir = app_state_dir().join("workspace");
        let legacy = default_dataset_path();
        let (workspace, status) = load_or_init(&dir, &legacy);
        let history = read_history(&dir);
        let active_env = if workspace.environments.is_empty() {
            None
        } else {
            Some(0)
        };
        let app = Self {
            workspace_dir: dir,
            workspace,
            expanded: HashSet::new(),
            tabs: Vec::new(),
            active: None,
            active_env,
            curl_import_text: String::new(),
            ws: None,
            runner: None,
            http_config: HttpClientConfig::default(),
            send_gen: 0,
            active_abort: None,
            pretty_json: true,
            response_tab: crate::gui::messages::ResponseTab::Body,
            response_search: String::new(),
            sidebar_px: 300.0,
            editor_px: 360.0,
            history,
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

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }
}
