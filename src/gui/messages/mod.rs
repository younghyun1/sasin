//! GUI message type for the workspace shell. Picker-choice enums live in [`choices`].

mod choices;

pub use choices::{AuthChoice, BodyModeChoice};

use iced::widget::text_editor;

use crate::model::{NodePath, RawLang, WsKind};
use crate::models::{HttpMethod, ResponseModel};
use crate::storage::HistoryRecord;
use crate::ws::WsEvent;

/// Monotonic id used to drop stale in-flight send results.
pub type SendGen = u64;

/// Identifies which split divider is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitId {
    Sidebar,
    RequestResponse,
}

/// Response panel sub-tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
    Cookies,
    Preview,
}

/// Editor sub-tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPanel {
    Params,
    Headers,
    Auth,
    Body,
    Scripts,
    Settings,
}

/// Which key/value table a [`KvOp`] targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvTarget {
    Params,
    Headers,
    UrlEncoded,
    FormData,
}

/// A row operation on a key/value (or form-data) table.
#[derive(Debug, Clone)]
pub enum KvOp {
    Add,
    Remove(usize),
    Key(usize, String),
    Value(usize, String),
    Toggle(usize, bool),
}

/// Which auth text field changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFieldKind {
    User,
    Pass,
    Token,
    ApiKeyName,
    ApiKeyValue,
}

/// A boolean per-request setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingFlag {
    FollowRedirects,
    VerifyTls,
    CookieJar,
}

/// Direction for an in-place sibling move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDir {
    Up,
    Down,
}

/// Tree structure operations (rename / duplicate / create / reorder), grouped so the
/// central dispatch stays a single thin arm.
#[derive(Debug, Clone)]
pub enum TreeMsg {
    RenameStart(NodePath),
    RenameInput(String),
    RenameCommit,
    RenameCancel,
    Duplicate(NodePath),
    /// Create a folder under the given parent (empty path = workspace root).
    NewFolder(NodePath),
    /// Create a request under the given parent (empty path = workspace root).
    NewRequestIn(NodePath),
    Move(NodePath, MoveDir),
}

#[derive(Debug, Clone)]
pub enum Message {
    // --- Collection tree ---
    ToggleFolder(NodePath),
    OpenNode(NodePath),
    NewRequest,
    DeleteNode(NodePath),
    Tree(TreeMsg),

    // --- Environments ---
    SelectEnv(usize),
    NewEnv,
    EnvVar(KvOp),

    // --- curl interop ---
    CurlImportChanged(String),
    CurlImport,
    CopyAsCurl,

    // --- WebSocket (NodePath identifies the session) ---
    Ws(NodePath, WsEvent),
    WsConnect,
    WsDisconnect,
    WsComposerChanged(String),
    WsKindChanged(WsKind),
    WsSend,
    WsSendSaved(usize),

    // --- Collection runner ---
    OpenRunner(NodePath),
    RunnerClose,
    RunnerIterations(String),
    RunnerDataPathChanged(String),
    RunnerLoadData,
    RunnerStart,
    RunnerStop,
    RunnerFinished(SendGen, Result<ResponseModel, String>),

    // --- Tabs ---
    SelectTab(usize),
    CloseTab(usize),
    CloseActiveTab,

    // --- Editor top bar + panel selection ---
    MethodChanged(HttpMethod),
    UrlChanged(String),
    SelectPanel(EditorPanel),

    // --- Key/value tables (params, headers, urlencoded, form-data) ---
    Kv(KvTarget, KvOp),

    // --- Auth ---
    AuthChanged(AuthChoice),
    AuthField(AuthFieldKind, String),
    AuthApiKeyInHeader(bool),

    // --- Body ---
    BodyModeChanged(BodyModeChoice),
    RawLangChanged(RawLang),
    BodyAction(text_editor::Action),
    GqlVarsAction(text_editor::Action),
    PreScriptAction(text_editor::Action),
    TestScriptAction(text_editor::Action),
    FormPartFile(usize, bool),
    FormPartSrc(usize, String),
    BinaryFileChanged(String),

    // --- Settings ---
    SettingTimeout(String),
    SettingFlagChanged(SettingFlag, bool),
    SettingProxy(String),

    // --- Persistence ---
    SaveActiveTab,
    Saved(Result<(), String>),

    // --- Sending ---
    SendPressed,
    CancelPressed,
    RequestFinished(SendGen, ResponseModel),
    RequestFailed(SendGen, String),

    // --- Response view (global) ---
    TogglePrettyJson,
    SelectResponseTab(ResponseTab),
    ResponseSearchChanged(String),
    FocusResponseSearch,
    SaveAsExample,
    SaveBodyToFile,
    CopyBody,

    // --- History ---
    HistoryOpen(HistoryRecord),
    HistoryFilterChanged(String),
    HistoryShowMore,
    HistoryClear,

    // --- Sidebar search ---
    TreeFilterChanged(String),

    // --- Cookie manager ---
    ToggleCookieManager,
    ClearCookies,

    // --- Filesystem watch ---
    /// The workspace directory changed on disk (e.g. git pull); reload if it differs.
    WorkspaceChanged,

    // --- Status / misc ---
    Notice(String),
    /// A no-op (used by fire-and-forget tasks that produce nothing to handle).
    Ignore,

    // --- Layout / preferences ---
    SplitDragged(SplitId, f32),
    ToggleTheme,
    WindowResized(iced::Size),
    WindowCloseRequested(iced::window::Id),
    /// Debounced tick that flushes dirty preferences to disk.
    ConfigFlushTick,
}
