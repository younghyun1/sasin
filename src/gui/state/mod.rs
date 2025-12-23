//! GUI state modules.
//!
//! This module exists to pull non-UI logic out of `gui/app.rs`:
//! - dataset <-> editor synchronization
//! - autosave/debounce helpers
//! - small state structs that would otherwise bloat the App type

pub mod dataset_sync;

pub use dataset_sync::{
    DatasetUiState, EditorDraft, apply_editor_to_selected_request, ensure_default_request_exists,
    headers_to_text, load_request_into_editor, parse_headers,
};
