//! GUI message type for the workspace shell.

use iced::widget::text_editor;

use crate::model::NodePath;
use crate::models::{HttpMethod, ResponseModel};

/// Monotonic id used to drop stale in-flight send results.
pub type SendGen = u64;

/// Identifies which split divider is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitId {
    /// Sidebar (collection tree) vs main area.
    Sidebar,
    /// Request editor (top) vs response (bottom).
    RequestResponse,
}

#[derive(Debug, Clone)]
pub enum Message {
    // --- Collection tree ---
    /// Expand/collapse a folder.
    ToggleFolder(NodePath),
    /// Open a request/websocket node in a tab.
    OpenNode(NodePath),
    /// Create a new HTTP request at the workspace root.
    NewRequest,
    /// Delete a node (and its file) from the workspace.
    DeleteNode(NodePath),

    // --- Tabs ---
    SelectTab(usize),
    CloseTab(usize),

    // --- Active-tab editor ---
    MethodChanged(HttpMethod),
    UrlChanged(String),
    HeadersChanged(String),
    BodyAction(text_editor::Action),

    // --- Persistence ---
    /// Persist the active tab to its TOML file.
    SaveActiveTab,
    /// Result of an async workspace save.
    Saved(Result<(), String>),

    // --- Sending ---
    SendPressed,
    CancelPressed,
    RequestFinished(SendGen, ResponseModel),
    RequestFailed(SendGen, String),

    // --- Response view toggles (global) ---
    TogglePrettyJson,
    ToggleShowHeaders,

    // --- Layout ---
    SplitDragged(SplitId, f32),
}
