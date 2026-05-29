//! Central message dispatch for [`App`]. Thin arms delegate to handlers in the sibling modules.

use iced::Task;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::messages::SplitId;
use crate::gui::state::Tab;
use crate::model::{Node, find_node};

impl App {
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
            Message::NewRequest => self.new_request(),
            Message::DeleteNode(path) => self.delete_path(path),
            Message::SelectEnv(idx) => {
                self.select_env(idx);
                Task::none()
            }
            Message::NewEnv => {
                self.new_env();
                self.save_task()
            }
            Message::EnvVar(op) => {
                self.apply_env_var(op);
                self.save_task()
            }
            Message::CurlImportChanged(text) => {
                self.curl_import_text = text;
                Task::none()
            }
            Message::CurlImport => self.import_curl(),
            Message::CopyAsCurl => self.copy_as_curl(),
            Message::Ws(event) => {
                self.ws_event(event);
                Task::none()
            }
            Message::WsConnect => {
                self.ws_connect();
                Task::none()
            }
            Message::WsDisconnect => {
                self.ws_disconnect();
                Task::none()
            }
            Message::WsComposerChanged(text) => {
                self.ws_composer_changed(text);
                Task::none()
            }
            Message::WsKindChanged(kind) => {
                self.ws_kind_changed(kind);
                Task::none()
            }
            Message::WsSend => {
                self.ws_send();
                Task::none()
            }
            Message::WsSendSaved(idx) => {
                self.ws_send_saved(idx);
                Task::none()
            }
            Message::OpenRunner(path) => {
                self.open_runner(path);
                Task::none()
            }
            Message::RunnerClose => {
                self.runner = None;
                Task::none()
            }
            Message::RunnerIterations(text) => {
                if let Some(r) = &mut self.runner {
                    r.iterations_text = text;
                }
                Task::none()
            }
            Message::RunnerDataPathChanged(text) => {
                if let Some(r) = &mut self.runner {
                    r.data_path = text;
                }
                Task::none()
            }
            Message::RunnerLoadData => {
                self.load_runner_data();
                Task::none()
            }
            Message::RunnerStart => self.runner_start(),
            Message::RunnerStop => {
                self.runner_stop();
                Task::none()
            }
            Message::RunnerFinished(send_id, result) => self.runner_finished(send_id, result),
            Message::SelectTab(i) => {
                if i < self.tabs.len() {
                    self.active = Some(i);
                }
                Task::none()
            }
            Message::CloseTab(i) => self.close_tab(i),
            Message::MethodChanged(method) => {
                self.set_method(method);
                Task::none()
            }
            Message::UrlChanged(url) => {
                self.set_url(url);
                Task::none()
            }
            Message::SelectPanel(panel) => {
                self.select_panel(panel);
                Task::none()
            }
            Message::Kv(target, op) => {
                self.apply_kv(target, op);
                Task::none()
            }
            Message::AuthChanged(choice) => {
                self.set_auth_choice(choice);
                Task::none()
            }
            Message::AuthField(kind, value) => {
                self.set_auth_field(kind, value);
                Task::none()
            }
            Message::AuthApiKeyInHeader(in_header) => {
                self.set_apikey_in_header(in_header);
                Task::none()
            }
            Message::BodyModeChanged(choice) => {
                self.set_body_mode(choice);
                Task::none()
            }
            Message::RawLangChanged(lang) => {
                self.set_raw_lang(lang);
                Task::none()
            }
            Message::BodyAction(action) => {
                self.body_action(action, false);
                Task::none()
            }
            Message::GqlVarsAction(action) => {
                self.body_action(action, true);
                Task::none()
            }
            Message::PreScriptAction(action) => {
                self.script_action(action, false);
                Task::none()
            }
            Message::TestScriptAction(action) => {
                self.script_action(action, true);
                Task::none()
            }
            Message::FormPartFile(index, is_file) => {
                self.set_form_kind(index, is_file);
                Task::none()
            }
            Message::FormPartSrc(index, src) => {
                self.set_form_src(index, src);
                Task::none()
            }
            Message::BinaryFileChanged(file) => {
                self.set_binary_file(file);
                Task::none()
            }
            Message::SettingTimeout(text) => {
                self.set_timeout(text);
                Task::none()
            }
            Message::SettingFlagChanged(flag, value) => {
                self.set_flag(flag, value);
                Task::none()
            }
            Message::SettingProxy(proxy) => {
                self.set_proxy(proxy);
                Task::none()
            }
            Message::SaveActiveTab => {
                // Structured fields and body text are already live on the node; just persist.
                if let Some(i) = self.active {
                    self.tabs[i].dirty = false;
                    self.tabs[i].error = None;
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
                if let Some(pos) = self
                    .tabs
                    .iter()
                    .position(|t| t.send_gen == send_id && t.sending)
                {
                    self.tabs[pos].sending = false;
                    self.tabs[pos].response = Some(resp);
                    self.tabs[pos].error = None;
                    self.run_test_script(pos);
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
            Message::SelectResponseTab(tab) => {
                self.response_tab = tab;
                Task::none()
            }
            Message::ResponseSearchChanged(text) => {
                self.response_search = text;
                Task::none()
            }
            Message::SaveAsExample => self.save_as_example(),
            Message::HistoryOpen(idx) => self.open_history(idx),
            Message::Notice(text) => {
                self.status = Some(text);
                Task::none()
            }
            Message::Ignore => Task::none(),
            Message::SplitDragged(id, px) => {
                match id {
                    SplitId::Sidebar => self.sidebar_px = px.clamp(220.0, 560.0),
                    SplitId::RequestResponse => self.editor_px = px.clamp(220.0, 900.0),
                }
                Task::none()
            }
        }
    }
}
