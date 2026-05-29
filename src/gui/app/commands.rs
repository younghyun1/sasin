//! Async commands: persisting the workspace and sending the active request.

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state;
use crate::model::{Node, find_node, find_node_mut, resolve_auth};
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

    /// Send the active tab's request via the exec layer.
    ///
    /// The tab's editor buffers are flushed into the workspace node first (so the send reflects
    /// current edits, including the full stored model — params, auth, body mode), auth inheritance
    /// is resolved up the folder chain, and a generation id is tracked to drop stale results.
    pub(super) fn send_active(&mut self) -> Task<Message> {
        let Some(i) = self.active else {
            return Task::none();
        };
        let path = self.tabs[i].path.clone();

        // Flush editor buffers into the node so the send reflects current edits.
        if let Some(node) = find_node_mut(&mut self.workspace.root, &path)
            && let Err(e) = state::apply_tab_to_node(&self.tabs[i], node)
        {
            let tab = &mut self.tabs[i];
            tab.error = Some(e);
            tab.response = None;
            return Task::none();
        }

        // Only HTTP requests are sent here (websocket sessions are interactive — P7).
        let request = match find_node(&self.workspace.root, &path) {
            Some(Node::Http(r)) => r.clone(),
            _ => {
                self.tabs[i].error = Some("This node is not an HTTP request.".to_string());
                return Task::none();
            }
        };
        let auth = resolve_auth(&self.workspace.root, &path);
        let base_dir = self.workspace_dir.clone();

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
            match crate::http::execute(&cfg, &request, &auth, &base_dir).await {
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
