use crate::models::{HttpMethod, ResponseModel};

/// Monotonic request identifier used to ignore stale task results.
pub type RequestId = u64;

/// Identifies which split divider is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitId {
    /// Sidebar (templates/history) vs main area.
    Sidebar,
    /// Request (top) vs response (bottom) inside the main area.
    RequestResponse,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// User selected a different HTTP method.
    MethodChanged(HttpMethod),

    /// User edited the URL input.
    UrlChanged(String),

    /// User edited the raw body editor (request body).
    BodyChanged(String),

    /// User edited the raw headers editor (one header per line: `Name: Value`).
    HeadersChanged(String),

    /// User clicked the "Send" button.
    SendPressed,

    /// Cancel the currently in-flight request.
    CancelPressed,

    /// Background task finished successfully with a response.
    RequestFinished(RequestId, ResponseModel),

    /// Background task failed.
    RequestFailed(RequestId, String),

    /// Clear the current response/error output.
    ClearPressed,

    /// Toggle pretty printing for JSON responses (best-effort).
    TogglePrettyJson,

    /// Toggle whether response headers are shown.
    ToggleShowHeaders,

    /// Select an entry from history to restore into the editor.
    HistorySelected(usize),

    /// Clear request history.
    ClearHistory,

    // --- Resizable panels (split panes) ---
    /// Dragging a split divider changed its pixel position.
    SplitDragged(SplitId, f32),

    // --- Autosave (dataset/templates) ---
    /// Toggle autosave on/off (persisted in config).
    ToggleAutosave,

    /// Fired after a short delay to perform a debounced autosave.
    AutosaveTick,

    // --- Dataset file flow (saved request templates) ---
    /// Begin "Open dataset" flow (show file picker / load screen).
    OpenDatasetPressed,

    /// A dataset file has been selected by the user (or cancelled).
    DatasetFileSelected(Option<std::path::PathBuf>),

    /// Load a dataset from disk (async).
    LoadDataset(std::path::PathBuf),

    /// Dataset load finished.
    DatasetLoaded(std::path::PathBuf, Result<(), String>),

    /// Save current dataset to disk (async).
    SaveDataset,

    /// "Save As" dataset flow.
    SaveDatasetAsPressed,

    /// A dataset save path has been selected by the user (or cancelled).
    DatasetSavePathSelected(Option<std::path::PathBuf>),

    /// Dataset save finished.
    DatasetSaved(std::path::PathBuf, Result<(), String>),

    /// Create a new template from the current editor state and select it.
    ///
    /// This supports the "immediate mutation" workflow: if no template is selected,
    /// the user can create one, then further edits mutate it immediately.
    NewTemplatePressed,

    /// User edited the template name draft in the editor.
    ///
    /// If a template is selected, this will typically rename it (immediate mutation).
    /// If no template is selected, this updates the draft name for the next "New Template".
    TemplateNameChanged(String),

    /// Create/update a named request template in the dataset from current editor state.
    SaveTemplatePressed,

    /// Rename a template.
    RenameTemplatePressed(u64, String),

    /// Delete a template from the dataset.
    DeleteTemplatePressed(u64),

    /// Select a template from the dataset to load into the editor.
    TemplateSelected(u64),
}
