//! Async commands: persisting the workspace and sending the active request.

use std::collections::HashSet;

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state::{self, Tab};
use crate::interop;
use crate::model::{Node, find_node, find_node_mut, resolve_auth};
use crate::runtime;
use crate::scripting;
use crate::storage::layout::unique_slug;
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

    /// Parse the curl-import buffer into a new request at the workspace root and open it.
    pub(super) fn import_curl(&mut self) -> Task<Message> {
        let text = std::mem::take(&mut self.curl_import_text);
        if text.trim().is_empty() {
            return Task::none();
        }
        match interop::from_curl(&text) {
            Ok(mut req) => {
                let mut taken: HashSet<String> = self
                    .workspace
                    .root
                    .iter()
                    .map(|n| n.slug().to_string())
                    .collect();
                let slug = unique_slug("imported", &mut taken);
                req.slug = slug.clone();
                self.workspace.root.push(Node::Http(req));
                let path = vec![slug];
                if let Some(node) = find_node(&self.workspace.root, &path) {
                    self.tabs.push(Tab::from_node(path, node));
                    self.active = Some(self.tabs.len() - 1);
                }
                self.save_task()
            }
            Err(e) => {
                self.status = Some(format!("curl import failed: {e}"));
                Task::none()
            }
        }
    }

    /// Copy the active request to the clipboard as a curl command.
    pub(super) fn copy_as_curl(&mut self) -> Task<Message> {
        let Some(i) = self.active else {
            return Task::none();
        };
        let path = self.tabs[i].path.clone();
        match find_node(&self.workspace.root, &path) {
            Some(Node::Http(r)) => {
                let curl = interop::to_curl(r);
                self.status = Some("Copied request as curl.".to_string());
                iced::clipboard::write(curl)
            }
            _ => Task::none(),
        }
    }

    /// Run the active tab's test script against its response, recording results on the tab.
    pub(super) fn run_test_script(&mut self, pos: usize) {
        let Some(path) = self.tabs.get(pos).map(|t| t.path.clone()) else {
            return;
        };
        let test = match find_node(&self.workspace.root, &path) {
            Some(Node::Http(r)) => r.scripts.test.clone(),
            _ => String::new(),
        };
        if test.trim().is_empty() {
            return;
        }
        let Some(resp) = self.tabs.get(pos).and_then(|t| t.response.clone()) else {
            return;
        };
        let env = self
            .active_env
            .and_then(|i| self.workspace.environments.get(i));
        let snapshot = runtime::VarContext::from_scopes(&self.workspace.globals, env).snapshot();
        let outcome = scripting::run_test(&test, &snapshot, &resp);
        if let Some(tab) = self.tabs.get_mut(pos) {
            tab.script_console.extend(outcome.console);
            tab.script_tests = outcome.tests;
            if outcome.error.is_some() {
                tab.script_error = outcome.error;
            }
        }
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

        // Flush buffered editor fields (body text, scripts) into the node before sending.
        if let Some(node) = find_node_mut(&mut self.workspace.root, &path) {
            state::sync_body(&self.tabs[i], node);
            state::sync_scripts(&self.tabs[i], node);
        }

        // Only HTTP requests are sent here (websocket sessions are interactive — P7).
        let mut request = match find_node(&self.workspace.root, &path) {
            Some(Node::Http(r)) => r.clone(),
            _ => {
                self.tabs[i].error = Some("This node is not an HTTP request.".to_string());
                return Task::none();
            }
        };
        // Resolve auth inheritance, then build the variable context (active env over globals).
        request.auth = resolve_auth(&self.workspace.root, &path);
        let env = self
            .active_env
            .and_then(|idx| self.workspace.environments.get(idx));
        let mut ctx = runtime::VarContext::from_scopes(&self.workspace.globals, env);

        // Reset per-send script output; run the pre-request script (may set variables).
        self.tabs[i].script_tests.clear();
        self.tabs[i].script_console.clear();
        self.tabs[i].script_error = None;
        if !request.scripts.pre_request.trim().is_empty() {
            let outcome = scripting::run_pre_request(&request.scripts.pre_request, &ctx.snapshot());
            for (k, v) in outcome.var_sets {
                ctx.set(k, v);
            }
            self.tabs[i].script_console = outcome.console;
            self.tabs[i].script_error = outcome.error;
        }

        // Interpolate variables, then resolve file paths against the workspace dir.
        let request = runtime::resolve_request(&request, &ctx);
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
            match crate::http::execute(&cfg, &request, &base_dir).await {
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
