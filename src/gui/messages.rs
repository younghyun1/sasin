//! GUI message type for the workspace shell, plus the small UI-choice enums used by pickers.

use std::fmt;

use iced::widget::text_editor;

use crate::model::{NodePath, RawLang, WsKind};
use crate::models::{HttpMethod, ResponseModel};
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

/// Body-mode picker choice (mirrors [`crate::model::Body`] without payloads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyModeChoice {
    None,
    Raw,
    UrlEncoded,
    FormData,
    Binary,
    GraphQl,
}

/// Auth-type picker choice (mirrors [`crate::model::Auth`] without payloads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthChoice {
    None,
    Inherit,
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
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

#[derive(Debug, Clone)]
pub enum Message {
    // --- Collection tree ---
    ToggleFolder(NodePath),
    OpenNode(NodePath),
    NewRequest,
    DeleteNode(NodePath),

    // --- Environments ---
    SelectEnv(usize),
    NewEnv,
    EnvVar(KvOp),

    // --- curl interop ---
    CurlImportChanged(String),
    CurlImport,
    CopyAsCurl,

    // --- WebSocket ---
    Ws(WsEvent),
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
    SaveAsExample,

    // --- Status / misc ---
    Notice(String),

    // --- Layout ---
    SplitDragged(SplitId, f32),
}

impl BodyModeChoice {
    pub const fn all() -> &'static [BodyModeChoice] {
        &[
            BodyModeChoice::None,
            BodyModeChoice::Raw,
            BodyModeChoice::UrlEncoded,
            BodyModeChoice::FormData,
            BodyModeChoice::Binary,
            BodyModeChoice::GraphQl,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            BodyModeChoice::None => "None",
            BodyModeChoice::Raw => "Raw",
            BodyModeChoice::UrlEncoded => "URL-encoded",
            BodyModeChoice::FormData => "Form-data",
            BodyModeChoice::Binary => "Binary",
            BodyModeChoice::GraphQl => "GraphQL",
        }
    }
}

impl fmt::Display for BodyModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl AuthChoice {
    pub const fn all() -> &'static [AuthChoice] {
        &[
            AuthChoice::Inherit,
            AuthChoice::None,
            AuthChoice::Basic,
            AuthChoice::Bearer,
            AuthChoice::ApiKey,
            AuthChoice::OAuth2,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            AuthChoice::None => "No Auth",
            AuthChoice::Inherit => "Inherit",
            AuthChoice::Basic => "Basic",
            AuthChoice::Bearer => "Bearer",
            AuthChoice::ApiKey => "API Key",
            AuthChoice::OAuth2 => "OAuth2",
        }
    }
}

impl fmt::Display for AuthChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
