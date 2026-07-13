//! Async commands: persisting the workspace and sending the active request.

use std::collections::HashSet;

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state::{self, Tab};
use crate::interop;
use crate::model::{Node, find_node, find_node_mut, resolve_auth};
use crate::models::ResponseModel;
use crate::runtime;
use crate::scripting;
use crate::storage::layout::unique_slug;
use crate::storage::{HistoryRecord, save_workspace, write_history};

impl App {
    /// Variable context for the node at `path`, layered Postman-style:
    /// globals < ancestor-folder (collection) variables < active environment.
    pub(super) fn var_context(&self, path: &[String]) -> runtime::VarContext {
        let mut ctx = runtime::VarContext::from_scopes(&self.workspace.globals, None);
        for scope in crate::model::folder_var_scopes(&self.workspace.root, path) {
            ctx.overlay_variables(scope);
        }
        if let Some(env) = self
            .active_env
            .and_then(|i| self.workspace.environments.get(i))
        {
            ctx.overlay_variables(&env.variables);
        }
        ctx
    }

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

    /// Pick a Postman v2.1 JSON export and parse it off-thread.
    pub(super) fn import_postman(&mut self) -> Task<Message> {
        Task::perform(
            async move {
                let Some(handle) = rfd::AsyncFileDialog::new()
                    .add_filter("Postman collection", &["json"])
                    .pick_file()
                    .await
                else {
                    return Message::Ignore;
                };
                let path = handle.path().to_path_buf();
                let parsed = tokio::task::spawn_blocking(move || {
                    std::fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|text| interop::from_postman(&text))
                })
                .await;
                match parsed {
                    Ok(Ok(import)) => Message::PostmanImported(Ok(Box::new(import))),
                    Ok(Err(e)) => Message::PostmanImported(Err(e)),
                    Err(e) => Message::PostmanImported(Err(e.to_string())),
                }
            },
            |m| m,
        )
    }

    /// Land a parsed Postman import: unique root slug, insert, expand, persist.
    pub(super) fn postman_imported(
        &mut self,
        result: Result<Box<interop::PostmanImport>, String>,
    ) -> Task<Message> {
        let import = match result {
            Ok(import) => *import,
            Err(e) => {
                self.status = Some(format!("Postman import failed: {e}"));
                return Task::none();
            }
        };
        let mut folder = import.folder;
        let mut taken = crate::model::sibling_slugs(&self.workspace.root, &[]);
        folder.slug = unique_slug(&folder.slug, &mut taken);
        let slug = folder.slug.clone();
        self.workspace.root.push(Node::Folder(folder));
        self.expanded.insert(vec![slug]);
        let mut status = format!("Imported {} request(s) from Postman.", import.request_count);
        if !import.warnings.is_empty() {
            status.push_str(&format!(
                " {} warning(s): {}",
                import.warnings.len(),
                import.warnings.first().map(String::as_str).unwrap_or("")
            ));
        }
        self.status = Some(status);
        self.save_task()
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

    /// Save the active tab's current response to `<request>.examples/` as a readable `.http` dump.
    pub(super) fn save_as_example(&mut self) -> Task<Message> {
        let Some(i) = self.active else {
            return Task::none();
        };
        let path = self.tabs[i].path.clone();
        let Some(resp) = self.tabs[i].response.clone() else {
            return notice("No response to save yet.");
        };
        let Some((slug, parents)) = path.split_last() else {
            return Task::none();
        };
        let mut dir = self.workspace_dir.clone();
        for segment in parents {
            dir.push(segment);
        }
        dir.push(format!("{slug}.examples"));
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let file = dir.join(format!("example-{stamp}.http"));
        let dump = format_example(&resp);

        Task::perform(
            async move {
                let written = tokio::task::spawn_blocking(move || {
                    std::fs::create_dir_all(&dir)
                        .and_then(|()| std::fs::write(&file, dump))
                        .map(|()| file.display().to_string())
                })
                .await;
                match written {
                    Ok(Ok(p)) => Message::Notice(format!("Saved example: {p}")),
                    Ok(Err(e)) => Message::Notice(format!("Example save failed: {e}")),
                    Err(e) => Message::Notice(format!("Example save failed: {e}")),
                }
            },
            |m| m,
        )
    }

    /// Save the active response body to a user-picked file (native save dialog).
    pub(super) fn save_body_to_file(&mut self) -> Task<Message> {
        let Some(i) = self.active else {
            return Task::none();
        };
        let Some(resp) = self.tabs[i].response.clone() else {
            return notice("No response to save yet.");
        };
        let url = match find_node(&self.workspace.root, &self.tabs[i].path) {
            Some(Node::Http(r)) => r.url.clone(),
            _ => String::new(),
        };
        let suggested = suggest_file_name(&url, &resp);
        Task::perform(
            async move {
                let Some(handle) = rfd::AsyncFileDialog::new()
                    .set_file_name(&suggested)
                    .save_file()
                    .await
                else {
                    return Message::Ignore;
                };
                let path = handle.path().to_path_buf();
                let bytes = resp.body.bytes().to_vec();
                let written =
                    tokio::task::spawn_blocking(move || std::fs::write(&path, bytes)).await;
                match written {
                    Ok(Ok(())) => {
                        Message::Notice(format!("Saved body: {}", handle.path().display()))
                    }
                    Ok(Err(e)) => Message::Notice(format!("Body save failed: {e}")),
                    Err(e) => Message::Notice(format!("Body save failed: {e}")),
                }
            },
            |m| m,
        )
    }

    /// Copy the active response body to the clipboard (lossy for binary).
    pub(super) fn copy_body(&mut self) -> Task<Message> {
        let Some(resp) = self.active.and_then(|i| self.tabs[i].response.as_ref()) else {
            return Task::none();
        };
        let body = resp.body.text_lossy().into_owned();
        self.status = Some("Copied response body.".to_string());
        iced::clipboard::write(body)
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
        let snapshot = self.var_context(&path).snapshot();
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
        // Resolve auth inheritance, then build the variable context
        // (globals < folder variables < active env).
        request.auth = resolve_auth(&self.workspace.root, &path);
        let mut ctx = self.var_context(&path);

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

        // Record the (resolved) request in persisted history.
        self.record_history(&request.method, &request.url);
        let history_task = self.persist_history();

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

        let send_task = Task::perform(
            async move {
                match join.await {
                    Ok(msg) => msg,
                    Err(_) => Message::RequestFailed(send_id, "Cancelled".to_string()),
                }
            },
            |msg| msg,
        );
        Task::batch([history_task, send_task])
    }

    /// Append a request to the in-memory history (capped, newest last).
    pub(super) fn record_history(&mut self, method: &str, url: &str) {
        let at_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
        self.history.records.push(HistoryRecord {
            method: method.to_string(),
            url: url.to_string(),
            at_unix_ms,
        });
        const CAP: usize = 200;
        let len = self.history.records.len();
        if len > CAP {
            self.history.records.drain(0..len - CAP);
        }
    }

    /// Persist the history cache off-thread (errors surface to the status line).
    pub(super) fn persist_history(&self) -> Task<Message> {
        let dir = self.workspace_dir.clone();
        let history = self.history.clone();
        Task::perform(
            async move {
                match tokio::task::spawn_blocking(move || write_history(&dir, &history)).await {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(e.to_string()),
                    Err(e) => Some(e.to_string()),
                }
            },
            |err| match err {
                Some(e) => Message::Notice(format!("History save failed: {e}")),
                None => Message::Ignore,
            },
        )
    }
}

/// A one-shot task that just sets the status line.
fn notice(message: &str) -> Task<Message> {
    let message = message.to_string();
    Task::perform(async move { message }, Message::Notice)
}

/// Suggest a save-file name: the URL's last path segment, else "response" + an extension
/// derived from the content type.
fn suggest_file_name(url: &str, resp: &ResponseModel) -> String {
    let segment = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|mut s| s.next_back().map(str::to_owned))
        })
        .filter(|s| !s.is_empty());
    if let Some(name) = &segment
        && name.contains('.')
    {
        return name.clone();
    }
    let base = segment.unwrap_or_else(|| "response".to_string());
    let content_type = resp
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ext = extension_for(content_type, resp.body.as_text().is_some());
    format!("{base}.{ext}")
}

fn extension_for(content_type: &str, is_text: bool) -> &'static str {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("json") {
        "json"
    } else if ct.contains("html") {
        "html"
    } else if ct.contains("xml") {
        "xml"
    } else if ct.contains("png") {
        "png"
    } else if ct.contains("jpeg") || ct.contains("jpg") {
        "jpg"
    } else if ct.contains("gif") {
        "gif"
    } else if ct.contains("svg") {
        "svg"
    } else if ct.contains("pdf") {
        "pdf"
    } else if is_text {
        "txt"
    } else {
        "bin"
    }
}

/// Render a response as a readable `.http`-style dump (status line, headers, blank line, body).
/// Binary bodies are noted instead of dumped (the dump is meant to be human-readable).
fn format_example(resp: &ResponseModel) -> String {
    let mut out = format!("HTTP {} {}\n", resp.status.code, resp.status.reason);
    for (name, value) in resp.headers.iter() {
        out.push_str(name.as_str());
        out.push_str(": ");
        out.push_str(value.to_str().unwrap_or("<non-utf8>"));
        out.push('\n');
    }
    out.push('\n');
    match resp.body.as_text() {
        Some(body) => out.push_str(body),
        None => out.push_str(&format!("<binary body: {} bytes>", resp.body.len())),
    }
    out
}
