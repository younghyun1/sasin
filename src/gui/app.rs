use iced::widget::{Space, button, column, container, row, text, text_editor};
use iced::{Element, Length, Task};

use crate::gui::Message;
use crate::gui::components::{
    RequestEditor, ResponseView, Section, Split, SplitAxis, TemplateList,
};
use crate::gui::state::{
    EditorDraft, apply_editor_to_selected_template, load_template_into_editor,
};
use crate::http::{self, HttpClientConfig};
use crate::models::{HttpMethod, RequestModel, ResponseModel};
use crate::persist::{
    AppConfig, AppConfigFile, Dataset, DatasetFile, DatasetId, LayoutState, RequestTemplate,
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

    // Resizable panel state (pixels)
    sidebar_width_px: f32,
    request_height_px: f32,

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

        let layout = cfg.layout.unwrap_or(LayoutState::default());

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

            // Resizable panels (pixels)
            sidebar_width_px: layout.sidebar_width_px as f32,
            request_height_px: layout.request_height_px as f32,

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
            Message::SplitDragged(split_id, new_px) => {
                // Clamp to sane minimums so panes don't collapse.
                match split_id {
                    crate::gui::messages::SplitId::Sidebar => {
                        self.sidebar_width_px = new_px.clamp(240.0, 520.0);
                    }
                    crate::gui::messages::SplitId::RequestResponse => {
                        self.request_height_px = new_px.clamp(220.0, 900.0);
                    }
                }
                // Persist layout immediately; this is lightweight.
                self.app_config.layout = Some(LayoutState {
                    sidebar_width_px: self.sidebar_width_px.round() as u32,
                    request_height_px: self.request_height_px.round() as u32,
                });
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

            Message::MethodChanged(method) => {
                self.request.method = method;
                return self.apply_editor_mutation_and_maybe_autosave();
            }
            Message::UrlChanged(url) => {
                self.request.url = url;
                return self.apply_editor_mutation_and_maybe_autosave();
            }
            Message::HeadersChanged(s) => {
                self.headers_text = s;
                return self.apply_editor_mutation_and_maybe_autosave();
            }
            Message::BodyChanged(s) => {
                self.body_editor = text_editor::Content::with_text(&s);
                return self.apply_editor_mutation_and_maybe_autosave();
            }

            Message::TemplateNameChanged(name) => {
                self.template_name_input = name;
                return self.apply_editor_mutation_and_maybe_autosave();
            }

            Message::NewTemplatePressed => {
                // Create a new template from current editor draft and select it.
                // This is the "like Postman" flow: if nothing is selected, you can materialize
                // the current editor into a saved template, then edits mutate it immediately.
                let id = self.dataset.next_id();

                let draft = self.editor_draft();
                let name = draft.template_name.trim();

                // Default name: "<METHOD> <host>" (or "<METHOD> <url>" as fallback).
                // If URL is not parseable, keep it simple and still deterministic.
                let default_name = match draft.url.parse::<reqwest::Url>() {
                    Ok(u) => {
                        let host = u.host_str().unwrap_or("unknown-host");
                        format!("{} {}", draft.method.as_str(), host)
                    }
                    Err(_) => {
                        let url = draft.url.trim();
                        if url.is_empty() {
                            format!("{} request", draft.method.as_str())
                        } else {
                            format!("{} {}", draft.method.as_str(), url)
                        }
                    }
                };

                let template_name = if name.is_empty() {
                    default_name
                } else {
                    name.to_string()
                };

                // Headers parsing must succeed to create a template.
                let headers = match crate::gui::state::parse_headers(&draft.headers_text) {
                    Ok(h) => h,
                    Err(e) => {
                        self.error = Some(e);
                        return Task::none();
                    }
                };

                let mut t =
                    RequestTemplate::new(id, template_name, draft.method, draft.url.clone());
                t.headers = headers;
                t.body = draft.body_option();

                self.dataset.upsert(t);
                self.selected_template = Some(id);
                self.dataset_dirty = true;

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

                let headers = match crate::gui::state::parse_headers(&self.headers_text) {
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
                if let Some(draft) = load_template_into_editor(&self.dataset, id) {
                    self.selected_template = Some(id);
                    self.template_name_input = draft.template_name;
                    self.request.method = draft.method;
                    self.request.url = draft.url;
                    self.headers_text = draft.headers_text;
                    self.body_editor = text_editor::Content::with_text(&draft.body_text);
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
                let headers = match crate::gui::state::parse_headers(&self.headers_text) {
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

        // --- Sidebar: templates + history (Postman-like left panel) ---
        let templates = TemplateList::new(&self.dataset)
            .selected(self.selected_template)
            .height(Length::Fixed(360.0))
            .view();

        let history = self.view_history_inline();

        let sidebar = column![
            Section::new("Templates", templates).into_element(),
            Section::new("History", history).into_element(),
            row![
                button(text("New Template").size(14))
                    .padding(10)
                    .on_press(Message::NewTemplatePressed),
                Space::new().width(Length::Fill),
                button(text("Clear History").size(14))
                    .padding(10)
                    .on_press(Message::ClearHistory),
            ]
            .spacing(10)
        ]
        .spacing(12.0)
        .width(Length::Fill)
        .height(Length::Fill);

        // --- Main area: request on top, response below (Postman-like) ---
        let request_editor = RequestEditor::new(
            self.request.method,
            &self.request.url,
            &self.headers_text,
            &self.body_editor,
            &self.template_name_input,
        )
        .autosave_enabled(self.app_config.autosave_enabled)
        .dataset_dirty(self.dataset_dirty)
        .sending(self.sending)
        .headers_height_px(120.0)
        .body_height_px(240.0)
        .view();

        let response_view = ResponseView::new()
            .response(self.response.as_ref())
            .error(self.error.as_deref())
            .show_headers(self.show_headers)
            .pretty_json(self.pretty_json)
            .body_text(None)
            .headers_height(Length::Fixed(160.0))
            .view();

        // Resizable inner split: request (top) vs response (bottom)
        let main: Element<'_, Message> = Split::new(SplitAxis::Vertical)
            .first(Section::untitled(request_editor).into_element())
            .second(Section::untitled(response_view).into_element())
            .split_px(self.request_height_px)
            .min_first_px(220.0)
            .min_second_px(200.0)
            .on_drag(|px| Message::SplitDragged(crate::gui::messages::SplitId::RequestResponse, px))
            .into();

        // Resizable outer split: sidebar (left) vs main (right)
        let content: Element<'_, Message> = Split::new(SplitAxis::Horizontal)
            .first(sidebar)
            .second(main)
            .split_px(self.sidebar_width_px)
            .min_first_px(240.0)
            .min_second_px(520.0)
            .on_drag(|px| Message::SplitDragged(crate::gui::messages::SplitId::Sidebar, px))
            .into();

        container(content)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
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

    /// Inline history rendering that only borrows from `self`.
    ///
    /// This avoids building a temporary `Vec` inside `view()` that would be dropped
    /// before the returned `Element` is used.
    fn view_history_inline(&self) -> Element<'_, Message> {
        if self.history.is_empty() {
            return text("No history yet.").size(14).into();
        }

        let mut col = column!().spacing(8.0).width(Length::Fill);

        for (idx, h) in self.history.iter().enumerate() {
            let label = format!("{} {}", h.method.as_str(), h.url);

            let mut b = button(text(label).size(13)).padding(10);

            if self.selected_history != Some(idx) {
                b = b.on_press(Message::HistorySelected(idx));
            }

            col = col.push(b);
        }

        container(col).width(Length::Fill).into()
    }

    fn mark_dataset_dirty_from_editor_change(&mut self) {
        if matches!(self.dataset_ui, DatasetUiState::Ready { .. }) {
            self.dataset_dirty = true;
        }
    }

    fn editor_draft(&self) -> EditorDraft {
        EditorDraft {
            method: self.request.method,
            url: self.request.url.clone(),
            headers_text: self.headers_text.clone(),
            body_text: self.body_editor.text(),
            template_name: self.template_name_input.clone(),
        }
    }

    fn apply_editor_mutation_and_maybe_autosave(&mut self) -> Task<Message> {
        // Immediate mutation:
        // If a template is selected, apply editor state into it right away.
        // If parsing fails, surface error and do not mutate.
        let draft = self.editor_draft();
        match apply_editor_to_selected_template(&mut self.dataset, self.selected_template, &draft) {
            Ok(updated) => {
                if updated {
                    self.dataset_dirty = true;
                }
            }
            Err(e) => {
                self.error = Some(e);
                return Task::none();
            }
        }

        self.mark_dirty_and_maybe_schedule_autosave()
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
