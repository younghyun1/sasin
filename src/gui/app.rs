use iced::alignment::Vertical;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_editor, text_input,
};
use iced::{Element, Length, Task};

use crate::gui::Message;
use crate::http::{self, HttpClientConfig};
use crate::models::{HeaderEntry, HttpMethod, RequestModel, ResponseModel};
use crate::persist::{
    AppConfig, AppConfigFile, Dataset, DatasetFile, DatasetId, RequestTemplate,
    default_config_path, default_dataset_path, load_startup_dataset,
};

#[derive(Debug, Clone)]
struct HistoryEntry {
    method: HttpMethod,
    url: String,
    headers_text: String,
    body_text: String,
}

impl HistoryEntry {
    fn from_state(app: &App) -> Self {
        Self {
            method: app.request.method,
            url: app.request.url.clone(),
            headers_text: app.headers_text.clone(),
            body_text: app.body_editor.text(),
        }
    }
}

#[derive(Debug, Clone)]
enum DatasetUiState {
    /// Loading dataset from disk.
    Loading { path: std::path::PathBuf },
    /// Ready with an open dataset file loaded.
    Ready { path: std::path::PathBuf },
    /// Error state; allow user to retry/open another file.
    Error { message: String },
}

#[derive(Debug)]
pub struct App {
    request: RequestModel,
    http_config: HttpClientConfig,

    // Editors (raw text)
    headers_text: String,
    body_editor: text_editor::Content,

    // UI toggles / options
    pretty_json: bool,
    show_headers: bool,

    // Async state
    sending: bool,
    response: Option<ResponseModel>,
    error: Option<String>,

    // Cancellation / staleness control (hard cancel + ignore late arrivals)
    active_request: Option<u64>,
    request_generation: u64,

    // Hard cancel support. Keep the AbortHandle so we can cancel even after
    // the join handle has been moved into a `Task::perform`.
    active_abort: Option<tokio::task::AbortHandle>,

    // History
    history: Vec<HistoryEntry>,
    selected_history: Option<usize>,

    // Dataset + config persistence (saved request templates + last-opened dataset)
    dataset_ui: DatasetUiState,
    dataset: Dataset,
    dataset_dirty: bool,
    selected_template: Option<DatasetId>,
    template_name_input: String,

    // Autosave (debounced)
    autosave_pending: bool,

    config_path: std::path::PathBuf,
    dataset_path: std::path::PathBuf,
    app_config: AppConfig,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        // Determine default paths (cross-platform) and load app config.
        let cfg_path = default_config_path();
        let ds_default = default_dataset_path();

        let cfg_file = AppConfigFile::new(&cfg_path);
        let cfg = cfg_file
            .load_or_default()
            .unwrap_or_else(|_| AppConfig::default());

        // Load last-opened dataset (or default dataset) and ensure at least one default template.
        // If startup load fails, keep the app usable but surface an error state.
        let (dataset_ui, ds_path, ds) = match load_startup_dataset(&cfg, ds_default.clone()) {
            Ok((p, d)) => (DatasetUiState::Ready { path: p.clone() }, p, d),
            Err(e) => (
                DatasetUiState::Error { message: e },
                ds_default,
                Dataset::default(),
            ),
        };

        Self {
            request: RequestModel::default(),
            http_config: HttpClientConfig::default(),
            headers_text: String::new(),
            body_editor: text_editor::Content::new(),
            pretty_json: true,
            show_headers: true,
            sending: false,
            response: None,
            error: None,
            active_request: None,
            request_generation: 0,
            active_abort: None,
            history: Vec::new(),
            selected_history: None,

            dataset_ui,
            dataset: ds,
            dataset_dirty: false,
            selected_template: None,
            template_name_input: String::new(),

            autosave_pending: false,

            config_path: cfg_path,
            dataset_path: ds_path,
            app_config: cfg,
        }
    }

    pub fn title(&self) -> String {
        "sasin — lean HTTP client".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MethodChanged(method) => {
                self.request.method = method;
                return self.mark_dirty_and_maybe_schedule_autosave();
            }
            Message::UrlChanged(url) => {
                self.request.url = url;
                return self.mark_dirty_and_maybe_schedule_autosave();
            }
            Message::HeadersChanged(s) => {
                self.headers_text = s;
                return self.mark_dirty_and_maybe_schedule_autosave();
            }
            Message::BodyChanged(s) => {
                self.body_editor = text_editor::Content::with_text(&s);
                return self.mark_dirty_and_maybe_schedule_autosave();
            }
            Message::TogglePrettyJson => {
                self.pretty_json = !self.pretty_json;
                Task::none()
            }
            Message::ToggleShowHeaders => {
                self.show_headers = !self.show_headers;
                Task::none()
            }
            Message::ToggleAutosave => {
                self.app_config.autosave_enabled = !self.app_config.autosave_enabled;

                // Persist config immediately
                let cfg_path = self.config_path.clone();
                let cfg = self.app_config.clone();
                return Task::perform(
                    async move {
                        let f = AppConfigFile::new(cfg_path);
                        f.save(&cfg).map_err(|e| e.to_string())
                    },
                    |res| match res {
                        Ok(_) => Message::ClearPressed,
                        Err(e) => Message::RequestFailed(0, e),
                    },
                );
            }
            Message::AutosaveTick => {
                self.autosave_pending = false;

                if !self.app_config.autosave_enabled || !self.dataset_dirty {
                    return Task::none();
                }

                return Task::perform(async {}, |_| Message::SaveDataset);
            }
            Message::ClearPressed => {
                self.response = None;
                self.error = None;
                Task::none()
            }
            Message::ClearHistory => {
                self.history.clear();
                self.selected_history = None;
                Task::none()
            }
            Message::HistorySelected(idx) => {
                if let Some(entry) = self.history.get(idx).cloned() {
                    self.selected_history = Some(idx);
                    self.request.method = entry.method;
                    self.request.url = entry.url;
                    self.headers_text = entry.headers_text;
                    self.body_editor = text_editor::Content::with_text(&entry.body_text);
                }
                Task::none()
            }

            // --- Dataset UI flow ---
            Message::OpenDatasetPressed => {
                self.dataset_ui = DatasetUiState::Loading {
                    path: self.dataset_path.clone(),
                };

                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter("sasin dataset", &["sasin"])
                            .pick_file()
                            .await
                            .map(|h| h.path().to_owned())
                    },
                    Message::DatasetFileSelected,
                );
            }
            Message::DatasetFileSelected(path_opt) => {
                if let Some(path) = path_opt {
                    self.dataset_ui = DatasetUiState::Loading { path: path.clone() };
                    Task::perform(async move { path }, Message::LoadDataset)
                } else {
                    // Cancelled: keep current dataset open if any
                    self.dataset_ui = DatasetUiState::Ready {
                        path: self.dataset_path.clone(),
                    };
                    Task::none()
                }
            }
            Message::LoadDataset(path) => {
                let p = path.clone();
                let p2 = p.clone();
                Task::perform(
                    async move {
                        let file = DatasetFile::new(&p);
                        match file.load_or_default() {
                            Ok(ds) => Ok((p2, ds)),
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    move |res| match res {
                        Ok((p, _ds)) => {
                            // We can't mutate state here; do it in DatasetLoaded
                            Message::DatasetLoaded(p, Ok(()))
                        }
                        Err(e) => Message::DatasetLoaded(path, Err(e)),
                    },
                )
            }
            Message::DatasetLoaded(path, result) => {
                match result {
                    Ok(()) => {
                        // Load dataset now and mark as ready.
                        let file = DatasetFile::new(&path);
                        match file.load_or_default() {
                            Ok(mut ds) => {
                                // Ensure default template exists.
                                if ds.templates.is_empty() {
                                    ds.templates.push(RequestTemplate::new(
                                        ds.next_id(),
                                        "Default",
                                        HttpMethod::Get,
                                        "https://example.com",
                                    ));
                                    let _ = file.save(&ds);
                                }

                                self.dataset = ds;
                                self.dataset_dirty = false;
                                self.dataset_ui = DatasetUiState::Ready { path: path.clone() };
                                self.dataset_path = path.clone();

                                // Persist "last opened dataset path" into app config.
                                self.app_config.last_dataset_path =
                                    Some(path.display().to_string());
                                let cfg_file = AppConfigFile::new(&self.config_path);
                                let _ = cfg_file.save(&self.app_config);

                                self.selected_template = None;
                                self.template_name_input.clear();
                            }
                            Err(e) => {
                                self.dataset_ui = DatasetUiState::Error {
                                    message: e.to_string(),
                                };
                            }
                        }
                    }
                    Err(e) => {
                        self.dataset_ui = DatasetUiState::Error { message: e };
                    }
                }
                Task::none()
            }
            Message::SaveDataset => {
                let path = match &self.dataset_ui {
                    DatasetUiState::Ready { path } => path.clone(),
                    _ => {
                        self.error = Some("No dataset file open. Use Save As.".to_string());
                        return Task::none();
                    }
                };

                let ds = self.dataset.clone();
                let path2 = path.clone();
                let path_for_err = path.clone();
                let path_for_task = path.clone();

                Task::perform(
                    async move {
                        let file = DatasetFile::new(&path_for_task);
                        file.save(&ds).map_err(|e| e.to_string())?;
                        Ok(path2)
                    },
                    move |res| match res {
                        Ok(p) => Message::DatasetSaved(p, Ok(())),
                        Err(e) => Message::DatasetSaved(path_for_err, Err(e)),
                    },
                )
            }
            Message::SaveDatasetAsPressed => {
                let suggested = self.dataset_path.clone();
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter("sasin dataset", &["sasin"])
                            .set_file_name(
                                suggested
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("dataset.sasin"),
                            )
                            .save_file()
                            .await
                            .map(|h| h.path().to_owned())
                    },
                    Message::DatasetSavePathSelected,
                );
            }
            Message::DatasetSavePathSelected(path_opt) => {
                if let Some(path) = path_opt {
                    let ds = self.dataset.clone();
                    let p = path.clone();
                    let p2 = p.clone();
                    Task::perform(
                        async move {
                            let file = DatasetFile::new(&p);
                            file.save(&ds).map_err(|e| e.to_string())?;
                            Ok(p2)
                        },
                        move |res| match res {
                            Ok(p) => Message::DatasetSaved(p, Ok(())),
                            Err(e) => Message::DatasetSaved(path, Err(e)),
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::DatasetSaved(path, res) => {
                match res {
                    Ok(()) => {
                        self.dataset_dirty = false;
                        self.dataset_ui = DatasetUiState::Ready { path };
                    }
                    Err(e) => {
                        self.dataset_ui = DatasetUiState::Error { message: e };
                    }
                }
                Task::none()
            }
            Message::SaveTemplatePressed => {
                // Require dataset open
                let Some(_path) = (match &self.dataset_ui {
                    DatasetUiState::Ready { path } => Some(path),
                    _ => None,
                }) else {
                    self.error = Some("Open a dataset file first.".to_string());
                    return Task::none();
                };

                let name = self.template_name_input.trim();
                if name.is_empty() {
                    self.error = Some("Template name is empty.".to_string());
                    return Task::none();
                }

                let headers = match parse_headers(&self.headers_text) {
                    Ok(h) => h,
                    Err(e) => {
                        self.error = Some(e);
                        return Task::none();
                    }
                };

                let body_text = self.body_editor.text();
                let body = if body_text.trim().is_empty() {
                    None
                } else {
                    Some(body_text)
                };

                let id = self
                    .selected_template
                    .unwrap_or_else(|| self.dataset.next_id());
                let mut t =
                    RequestTemplate::new(id, name, self.request.method, self.request.url.clone());
                t.headers = headers;
                t.body = body;
                self.dataset.upsert(t);
                self.dataset_dirty = true;
                self.selected_template = Some(id);

                // Persist immediately for "persistently update it"
                Task::perform(async move { () }, |_| Message::SaveDataset)
            }
            Message::TemplateSelected(id) => {
                if let Some(t) = self.dataset.templates.iter().find(|t| t.id == id).cloned() {
                    self.selected_template = Some(id);
                    self.template_name_input = t.name.clone();
                    self.request.method = t.method;
                    self.request.url = t.url;
                    self.headers_text = headers_to_text(&t.headers);
                    self.body_editor =
                        text_editor::Content::with_text(t.body.as_deref().unwrap_or(""));
                }
                Task::none()
            }
            Message::DeleteTemplatePressed(id) => {
                if self.dataset.remove(id) {
                    if self.selected_template == Some(id) {
                        self.selected_template = None;
                        self.template_name_input.clear();
                    }
                    self.dataset_dirty = true;
                    return Task::perform(async move { () }, |_| Message::SaveDataset);
                }
                Task::none()
            }
            Message::RenameTemplatePressed(id, new_name) => {
                let new_name = new_name.trim();
                if new_name.is_empty() {
                    return Task::none();
                }
                if let Some(t) = self.dataset.templates.iter().find(|t| t.id == id).cloned() {
                    let mut t2 = t;
                    t2.name = new_name.to_string();
                    self.dataset.upsert(t2);
                    self.dataset_dirty = true;
                    return Task::perform(async move { () }, |_| Message::SaveDataset);
                }
                Task::none()
            }

            // --- Request sending ---
            Message::CancelPressed => {
                // Hard cancel: abort the in-flight Tokio task.
                if let Some(abort) = self.active_abort.take() {
                    abort.abort();
                }

                self.active_request = None;
                self.sending = false;

                self.error = Some("Cancelled".to_string());
                Task::none()
            }
            Message::SendPressed => {
                self.response = None;
                self.error = None;

                // Abort any existing in-flight request before starting a new one.
                if let Some(abort) = self.active_abort.take() {
                    abort.abort();
                }

                // Build request model from the UI raw text.
                let headers = match parse_headers(&self.headers_text) {
                    Ok(h) => h,
                    Err(e) => {
                        self.error = Some(e);
                        return Task::none();
                    }
                };

                let body_text = self.body_editor.text();
                self.request.headers = headers;
                self.request.body = if body_text.trim().is_empty() {
                    None
                } else {
                    Some(body_text)
                };

                if let Err(e) = self.request.validate() {
                    self.error = Some(e.to_string());
                    return Task::none();
                }

                self.sending = true;

                // Push into history (most recent first).
                self.history.insert(0, HistoryEntry::from_state(self));
                self.selected_history = Some(0);

                // New request id; used to ignore stale completion after cancel / superseding.
                self.request_generation = self.request_generation.wrapping_add(1);
                let req_id = self.request_generation;
                self.active_request = Some(req_id);

                let cfg = self.http_config.clone();
                let req = self.request.clone();

                let join = tokio::spawn(async move {
                    match http::send(&cfg, req).await {
                        Ok(resp) => Message::RequestFinished(req_id, resp),
                        Err(err) => Message::RequestFailed(req_id, err),
                    }
                });

                self.active_abort = Some(join.abort_handle());

                Task::perform(
                    async move {
                        match join.await {
                            Ok(msg) => Ok(msg),
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    move |res| match res {
                        Ok(msg) => msg,
                        Err(_e) => Message::RequestFailed(req_id, "Cancelled".to_string()),
                    },
                )
            }
            Message::RequestFinished(req_id, resp) => {
                if self.active_request != Some(req_id) {
                    return Task::none();
                }

                self.sending = false;
                self.active_request = None;
                self.active_abort = None;
                self.response = Some(resp);
                self.error = None;
                Task::none()
            }
            Message::RequestFailed(req_id, err) => {
                if self.active_request != Some(req_id) {
                    return Task::none();
                }

                self.sending = false;
                self.active_request = None;
                self.active_abort = None;
                self.response = None;
                self.error = Some(err);
                Task::none()
            } // Messages not wired in this file (file dialogs should send these)
              // (handled above)
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Startup dataset loading / error UI (we auto-load; no "gate" step)
        match &self.dataset_ui {
            DatasetUiState::Loading { path } => {
                return self.view_dataset_loading(path);
            }
            DatasetUiState::Error { message } => {
                return self.view_dataset_error(message);
            }
            DatasetUiState::Ready { .. } => {
                // fall through to main UI
            }
        }

        let left = self.view_request_panel();
        let right = self.view_response_panel();

        let content = row![left, right]
            .spacing(14)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_request_panel(&self) -> Element<'_, Message> {
        let top = self.view_top_bar();

        let section_gap = || Space::new().height(Length::Fixed(10.0));

        let headers_input = text_input(
            "Header: Value (one per line). Example: Accept: application/json",
            &self.headers_text,
        )
        .on_input(Message::HeadersChanged)
        .padding(12)
        .size(14)
        .width(Length::Fill);

        // Make body a fixed-height box (text_editor has internal scrolling),
        // so it looks like a complete editor "card" in the UI.
        let body_box = text_editor(&self.body_editor)
            .placeholder("Request body (raw text)…")
            .on_action(|action| {
                let mut content = self.body_editor.clone();
                content.perform(action);
                Message::BodyChanged(content.text())
            })
            .height(260.0);

        let cancel_btn = button(text("Cancel").size(14))
            .padding(10)
            .on_press_maybe(self.sending.then_some(Message::CancelPressed));

        let clear_btn = button(text("Clear Output").size(14))
            .padding(10)
            .on_press(Message::ClearPressed);

        let save_template_btn = button(text("Save Template").size(14))
            .padding(10)
            .on_press(Message::SaveTemplatePressed);

        let save_dataset_btn = button(text("Save Dataset").size(14))
            .padding(10)
            .on_press(Message::SaveDataset);

        let clear_hist_btn = button(text("Clear History").size(14))
            .padding(10)
            .on_press(Message::ClearHistory);

        let history = self.view_history();
        let templates = self.view_templates();

        let template_name = text_input("Template name…", &self.template_name_input)
            .on_input(|s| Message::RenameTemplatePressed(self.selected_template.unwrap_or(0), s))
            .padding(12)
            .size(14)
            .width(Length::Fill);

        let autosave_label = if self.app_config.autosave_enabled {
            "Autosave: On"
        } else {
            "Autosave: Off"
        };

        let content = column![
            top,
            section_gap(),
            row![text("Dataset").size(16),]
                .spacing(10)
                .align_y(Vertical::Center),
            row![
                button(text("Open…").size(14))
                    .padding(10)
                    .on_press(Message::OpenDatasetPressed),
                button(text("Save As…").size(14))
                    .padding(10)
                    .on_press(Message::SaveDatasetAsPressed),
                button(text(autosave_label).size(14))
                    .padding(10)
                    .on_press(Message::ToggleAutosave),
            ]
            .spacing(10)
            .align_y(Vertical::Center),
            section_gap(),
            text("Templates").size(16),
            templates,
            section_gap(),
            text("Template Name").size(16),
            template_name,
            section_gap(),
            text("Headers").size(16),
            headers_input,
            section_gap(),
            text("Body").size(16),
            body_box,
            section_gap(),
            row![cancel_btn, clear_btn, save_template_btn, save_dataset_btn]
                .spacing(10)
                .align_y(Vertical::Center),
            Space::new().height(Length::Fixed(14.0)),
            text("History").size(16),
            history,
            Space::new().height(Length::Fixed(6.0)),
            row![clear_hist_btn].spacing(10),
        ]
        .spacing(10)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        container(content)
            .padding(14)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .into()
    }

    fn view_top_bar(&self) -> Element<'_, Message> {
        let method_dropdown = pick_list(
            HttpMethod::all(),
            Some(self.request.method),
            Message::MethodChanged,
        )
        .padding(10);

        let url_input = text_input("https://example.com", &self.request.url)
            .on_input(Message::UrlChanged)
            .padding(12)
            .size(16)
            .width(Length::Fill);

        let send_label = if self.sending { "Sending…" } else { "Send" };
        let send_btn = button(text(send_label).size(16))
            .padding(12)
            .on_press_maybe((!self.sending).then_some(Message::SendPressed));

        row![method_dropdown, url_input, send_btn]
            .spacing(12)
            .align_y(Vertical::Center)
            .into()
    }

    fn view_history(&self) -> Element<'_, Message> {
        if self.history.is_empty() {
            return container(text("No history yet.").size(14))
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        let mut col = column!().spacing(8).width(Length::Fill);

        for (idx, h) in self.history.iter().enumerate() {
            let label = format!("{} {}", h.method.as_str(), h.url);

            let mut b = button(text(label).size(13)).padding(10);

            if self.selected_history != Some(idx) {
                b = b.on_press(Message::HistorySelected(idx));
            }

            col = col.push(b);
        }

        scrollable(col)
            .height(Length::Fixed(200.0))
            .width(Length::Fill)
            .into()
    }

    fn view_templates(&self) -> Element<'_, Message> {
        if self.dataset.templates.is_empty() {
            return container(text("No templates saved yet.").size(14))
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        let mut col = column!().spacing(8).width(Length::Fill);

        for t in &self.dataset.templates {
            let label = format!("{} • {} {}", t.name, t.method.as_str(), t.url);
            let mut b = button(text(label).size(13)).padding(10);
            if self.selected_template != Some(t.id) {
                b = b.on_press(Message::TemplateSelected(t.id));
            }
            col = col.push(
                row![
                    b,
                    button(text("Del").size(12))
                        .padding(8)
                        .on_press(Message::DeleteTemplatePressed(t.id))
                ]
                .spacing(8),
            );
        }

        scrollable(col)
            .height(Length::Fixed(220.0))
            .width(Length::Fill)
            .into()
    }

    fn view_dataset_loading(&self, path: &std::path::Path) -> Element<'_, Message> {
        let content = column![
            text("Loading dataset…").size(24),
            Space::new().height(Length::Fixed(8.0)),
            text(path.display().to_string()).size(14),
            Space::new().height(Length::Fixed(18.0)),
            button(text("Cancel").size(16))
                .padding(12)
                .on_press(Message::OpenDatasetPressed),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_dataset_error<'a>(&self, message: &'a str) -> Element<'a, Message> {
        let content = column![
            text("Dataset error").size(24),
            Space::new().height(Length::Fixed(8.0)),
            text(message).size(14),
            Space::new().height(Length::Fixed(18.0)),
            row![
                button(text("Open dataset…").size(16))
                    .padding(12)
                    .on_press(Message::OpenDatasetPressed),
            ]
            .spacing(12),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_response_panel(&self) -> Element<'_, Message> {
        let pretty_btn = button(text(if self.pretty_json {
            "Pretty JSON: on"
        } else {
            "Pretty JSON: off"
        }))
        .padding(8)
        .on_press(Message::TogglePrettyJson);

        let headers_btn = button(text(if self.show_headers {
            "Show headers: on"
        } else {
            "Show headers: off"
        }))
        .padding(8)
        .on_press(Message::ToggleShowHeaders);

        let options = row![pretty_btn, headers_btn]
            .spacing(8)
            .align_y(Vertical::Center);

        let body = self.view_response_area();

        let content = column![
            row![text("Response").size(18),].align_y(Vertical::Center),
            options,
            body
        ]
        .spacing(10)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        container(content)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .into()
    }

    fn view_response_area(&self) -> Element<'_, Message> {
        if let Some(err) = &self.error {
            return container(
                column![
                    text("Error").size(16),
                    Space::new().height(Length::Fixed(4.0)),
                    text(err).size(14),
                ]
                .spacing(6),
            )
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        if let Some(resp) = &self.response {
            // Response stats: duration uses `:?` formatting.
            let stats = format!(
                "Status: {} {} • Duration: {:?} • Body: {} bytes",
                resp.status.code,
                resp.status.reason,
                resp.duration,
                resp.body.len()
            );

            let headers_block = if self.show_headers {
                let mut s = String::new();
                for (name, value) in resp.headers.iter() {
                    let value_str = value.to_str().unwrap_or("<non-utf8>");
                    s.push_str(name.as_str());
                    s.push_str(": ");
                    s.push_str(value_str);
                    s.push('\n');
                }
                if s.is_empty() {
                    "<no headers>".to_string()
                } else {
                    s
                }
            } else {
                String::new()
            };

            let body_text = format_response_body(&resp.body, self.pretty_json);

            let mut panel = column![text(stats).size(14)]
                .spacing(8)
                .width(Length::Fill)
                .height(Length::Fill);

            if self.show_headers {
                panel = panel.push(text("Headers").size(16)).push(
                    scrollable(text(headers_block).size(12))
                        .height(Length::Fixed(160.0))
                        .width(Length::Fill),
                );
            }

            panel = panel.push(text("Body").size(16)).push(
                scrollable(text(body_text).size(12))
                    .height(Length::Fill)
                    .width(Length::Fill),
            );

            return container(panel)
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        container(
            column![
                text("Ready").size(18),
                text("Enter a URL, pick a method, edit headers/body, and press Send.").size(14),
            ]
            .spacing(6),
        )
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn parse_headers(raw: &str) -> Result<Vec<HeaderEntry>, String> {
    let mut out = Vec::new();

    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(format!(
                "Invalid header on line {} (expected `Name: Value`): {line}",
                line_no + 1
            ));
        };

        let name = name.trim();
        let value = value.trim();

        if name.is_empty() {
            return Err(format!("Empty header name on line {}", line_no + 1));
        }

        out.push(HeaderEntry {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    Ok(out)
}

fn headers_to_text(headers: &[HeaderEntry]) -> String {
    let mut s = String::new();
    for h in headers {
        if h.name.trim().is_empty() {
            continue;
        }
        s.push_str(h.name.trim());
        s.push_str(": ");
        s.push_str(h.value.trim());
        s.push('\n');
    }
    s
}

fn format_response_body(body: &str, pretty_json: bool) -> String {
    if !pretty_json {
        return body.to_string();
    }

    // Best-effort JSON pretty print.
    // If it fails, fall back to the original body unchanged.
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| body.to_string()),
        Err(_) => body.to_string(),
    }
}

impl App {
    fn mark_dataset_dirty_from_editor_change(&mut self) {
        if matches!(self.dataset_ui, DatasetUiState::Ready { .. }) {
            self.dataset_dirty = true;
        }
    }

    fn mark_dirty_and_maybe_schedule_autosave(&mut self) -> Task<Message> {
        self.mark_dataset_dirty_from_editor_change();

        if !self.app_config.autosave_enabled {
            return Task::none();
        }

        // Debounce autosave: only schedule if not already pending.
        if self.autosave_pending {
            return Task::none();
        }

        self.autosave_pending = true;

        Task::perform(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            },
            |_| Message::AutosaveTick,
        )
    }
}
