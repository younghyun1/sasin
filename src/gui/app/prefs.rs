//! UI-preference handling: window/layout tracking, theme toggle, and debounced config flushes.

use iced::{Size, Task, window};
use tracing::warn;

use crate::gui::Message;
use crate::gui::app::App;
use crate::persist::{UiPrefs, save_prefs};

impl App {
    /// Copy live layout state into the preference struct before a flush.
    fn sync_prefs(&mut self) {
        self.prefs.layout.sidebar_px = self.sidebar_px;
        self.prefs.layout.editor_px = self.editor_px;
    }

    pub(super) fn toggle_theme(&mut self) -> Task<Message> {
        self.prefs.theme = self.prefs.theme.flipped();
        // A deliberate choice, not a drag: flush immediately.
        self.flush_config()
    }

    pub(super) fn window_resized(&mut self, size: Size) {
        // Maximized geometry also lands here; `maximized` is captured separately at close,
        // so a maximized session restores maximized regardless of the stored size.
        self.prefs.window.width = size.width;
        self.prefs.window.height = size.height;
        self.config_dirty = true;
    }

    /// Write the preferences off-thread; the 2s subscription tick calls this while dirty.
    pub(super) fn flush_config(&mut self) -> Task<Message> {
        self.sync_prefs();
        self.config_dirty = false;
        let prefs = self.prefs;
        Task::future(async move {
            save_prefs_blocking(prefs).await;
            Message::Ignore
        })
    }

    /// Close was requested (interception enabled via `exit_on_close_request(false)`):
    /// capture maximized state, flush the config, then actually close.
    pub(super) fn close_requested(&mut self, id: window::Id) -> Task<Message> {
        self.sync_prefs();
        let prefs = self.prefs;
        window::is_maximized(id).then(move |maximized| {
            let mut prefs = prefs;
            prefs.window.maximized = maximized;
            Task::future(async move { save_prefs_blocking(prefs).await })
                .then(move |()| window::close(id))
        })
    }
}

async fn save_prefs_blocking(prefs: UiPrefs) {
    if let Err(e) = tokio::task::spawn_blocking(move || save_prefs(&prefs)).await {
        warn!(error = %e, "Config save task panicked");
    }
}
