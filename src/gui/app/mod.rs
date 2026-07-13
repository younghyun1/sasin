//! Top-level application: owns the workspace + open tabs.
//!
//! Behavior is split across child modules (they may use `App`'s private fields): [`boot`] loads the
//! workspace, [`update`] dispatches messages, [`edit`]/[`edit_body`] mutate the active node,
//! [`commands`] runs async send/save, [`ws`] drives websocket sessions, and [`view`] renders.

mod boot;
mod commands;
mod edit;
mod edit_body;
mod keys;
mod nav;
mod prefs;
mod runner;
mod subscriptions;
mod tree_ops;
mod update;
mod view;
mod ws;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::Task;

use crate::gui::Message;
use crate::gui::runner_state::RunnerState;
use crate::gui::state::{CookieDraft, Tab, WsRuntime};
use crate::http::HttpClientConfig;
use crate::model::{NodePath, Workspace};
use crate::persist::{UiPrefs, app_state_dir, default_dataset_path};
use crate::storage::{HistoryCache, read_cookies, read_history};

use boot::load_or_init;

/// Initial number of history rows; "more" grows it by [`HISTORY_SHOWN_STEP`].
const HISTORY_SHOWN_DEFAULT: usize = 10;
const HISTORY_SHOWN_STEP: usize = 25;

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
    /// Concurrent websocket sessions, keyed by node path.
    ws: Vec<WsRuntime>,
    /// The active collection-runner session, if any.
    runner: Option<RunnerState>,
    http_config: HttpClientConfig,
    send_gen: u64,
    active_abort: Option<tokio::task::AbortHandle>,
    pretty_json: bool,
    response_tab: crate::gui::messages::ResponseTab,
    response_search: String,
    /// Whether the cookie-manager view occupies the main area.
    show_cookies: bool,
    sidebar_px: f32,
    editor_px: f32,
    /// Persisted, recently-sent requests (newest last); shown in the sidebar.
    history: HistoryCache,
    status: Option<String>,
    /// Persisted UI preferences (theme, window, layout).
    prefs: UiPrefs,
    /// Whether `prefs` has unsaved changes (drives the flush-tick subscription).
    config_dirty: bool,
    /// In-flight tree rename: the target path and the edit buffer.
    renaming: Option<(NodePath, String)>,
    /// Sidebar search filter; non-empty swaps the tree for a flat match list.
    tree_filter: String,
    /// History filter + visible-row cap (grown by "more").
    history_filter: String,
    history_shown: usize,
    /// Cookie-manager add-row buffers.
    cookie_draft: CookieDraft,
}

impl App {
    /// Boot: locate the workspace directory, loading/migrating/initializing as needed.
    pub fn new(prefs: UiPrefs) -> (Self, Task<Message>) {
        let dir = app_state_dir().join("workspace");
        let legacy = default_dataset_path();
        let (workspace, status) = load_or_init(&dir, &legacy);
        let history = read_history(&dir);
        // Restore the persisted cookie jar (best-effort; a bad file just starts empty).
        let http_config = HttpClientConfig::default();
        if let Some(bytes) = read_cookies(&dir)
            && let Err(e) = http_config.jar.load_json(&bytes)
        {
            tracing::warn!(error = %e, "Failed to restore cookie jar; starting empty");
        }
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
            ws: Vec::new(),
            runner: None,
            http_config,
            send_gen: 0,
            active_abort: None,
            pretty_json: true,
            response_tab: crate::gui::messages::ResponseTab::Body,
            response_search: String::new(),
            show_cookies: false,
            sidebar_px: prefs.layout.sidebar_px,
            editor_px: prefs.layout.editor_px,
            history,
            status,
            prefs,
            config_dirty: false,
            renaming: None,
            tree_filter: String::new(),
            history_filter: String::new(),
            history_shown: HISTORY_SHOWN_DEFAULT,
            cookie_draft: CookieDraft::default(),
        };
        (app, Task::none())
    }

    /// The active theme (bound via `Application::theme`).
    pub fn theme(&self) -> iced::Theme {
        crate::gui::theme::app_theme(self.prefs.theme)
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
