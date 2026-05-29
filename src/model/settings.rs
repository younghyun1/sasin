//! Per-request transport settings for HTTP and WebSocket.

use serde::{Deserialize, Serialize};

use crate::model::defaults::{default_connect_timeout_ms, default_timeout_ms, default_true};

/// HTTP request settings. Defaults match `WorkspaceDefaults`; when equal to the default
/// the whole `[settings]` table is omitted from the request file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_true")]
    pub verify_tls: bool,
    #[serde(default = "default_true")]
    pub use_cookie_jar: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            follow_redirects: true,
            verify_tls: true,
            use_cookie_jar: true,
            proxy: None,
        }
    }
}

impl Settings {
    /// True when equal to the default, used to omit `[settings]` from TOML.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// WebSocket connection settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WsSettings {
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default)]
    pub auto_reconnect: bool,
    #[serde(default = "default_true")]
    pub verify_tls: bool,
}

impl Default for WsSettings {
    fn default() -> Self {
        Self {
            connect_timeout_ms: default_connect_timeout_ms(),
            auto_reconnect: false,
            verify_tls: true,
        }
    }
}

impl WsSettings {
    /// True when equal to the default, used to omit `[settings]` from TOML.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
