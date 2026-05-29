//! Async commands: persisting the workspace and sending the active request.

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state;
use crate::http;
use crate::storage::save_workspace;

impl App {
    /// Persist the whole workspace to disk on a blocking thread.
    pub(super) fn save_task(&self) -> Task<Message> {
        let dir = self.workspace_dir.clone();
        let ws = self.workspace.clone();
        Task::perform(
            async move {
                match tokio::task::spawn_blocking(move || {
                    save_workspace(&dir, &ws).map_err(|e| e.to_string())
                })
                .await
                {
                    Ok(result) => result,
                    Err(e) => Err(e.to_string()),
                }
            },
            Message::Saved,
        )
    }

    /// Send the active tab's request, tracking a generation id to drop stale results.
    pub(super) fn send_active(&mut self) -> Task<Message> {
        let Some(i) = self.active else {
            return Task::none();
        };

        let model = match state::build_request_model(&self.tabs[i]) {
            Ok(m) => m,
            Err(e) => {
                let tab = &mut self.tabs[i];
                tab.error = Some(e);
                tab.response = None;
                return Task::none();
            }
        };

        if let Some(abort) = self.active_abort.take() {
            abort.abort();
        }

        self.send_gen += 1;
        let send_id = self.send_gen;
        {
            let tab = &mut self.tabs[i];
            tab.send_gen = send_id;
            tab.sending = true;
            tab.error = None;
            tab.response = None;
        }

        let cfg = self.http_config.clone();
        let join = tokio::spawn(async move {
            match http::send(&cfg, model).await {
                Ok(resp) => Message::RequestFinished(send_id, resp),
                Err(err) => Message::RequestFailed(send_id, err),
            }
        });
        self.active_abort = Some(join.abort_handle());

        Task::perform(
            async move {
                match join.await {
                    Ok(msg) => msg,
                    Err(_) => Message::RequestFailed(send_id, "Cancelled".to_string()),
                }
            },
            |msg| msg,
        )
    }
}
