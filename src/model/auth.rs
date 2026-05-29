//! Authentication config for requests, folders, and websocket sessions.
//!
//! Serialized as an internally-tagged TOML table keyed by `type`:
//! ```toml
//! [auth]
//! type = "bearer"
//! token = "{{access_token}}"
//! ```
//! `inherit` walks up the folder chain to the workspace default; `none` disables auth.

use serde::{Deserialize, Serialize};

/// Where an API key is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLoc {
    #[default]
    Header,
    Query,
}

/// Auth strategy. Defaults to [`Auth::Inherit`] so a new request adopts its folder's auth.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Auth {
    /// No authentication.
    None,
    /// Inherit from the enclosing folder / workspace default.
    #[default]
    Inherit,
    /// HTTP Basic.
    Basic { user: String, pass: String },
    /// `Authorization: Bearer <token>`.
    Bearer { token: String },
    /// API key in a header or query parameter.
    #[serde(rename = "apikey")]
    ApiKey {
        key: String,
        value: String,
        #[serde(default)]
        add_to: ApiKeyLoc,
    },
    /// OAuth2 — currently the resolved access token; full grant flows land later.
    #[serde(rename = "oauth2")]
    OAuth2 { token: String },
}

impl Auth {
    /// True when this is [`Auth::Inherit`] (the default), used to omit `[auth]` from TOML.
    pub fn is_inherit(&self) -> bool {
        matches!(self, Auth::Inherit)
    }
}
