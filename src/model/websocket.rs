//! WebSocket request definition (`*.ws.toml`).
//!
//! Like [`crate::model::request`], scalar/inline fields (`name`, `description`, `url`,
//! `subprotocols`) precede the `[auth]`/`[settings]` tables and `[[header]]`/`[[message]]`
//! arrays so the TOML serializer accepts the ordering.

use serde::{Deserialize, Serialize};

use crate::model::auth::Auth;
use crate::model::kv::KvEntry;
use crate::model::settings::WsSettings;

/// Encoding of a saved outbound message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsKind {
    #[default]
    Text,
    Binary,
    Json,
}

/// A saved outbound message that can be replayed during a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WsMessageTemplate {
    pub name: String,
    #[serde(default)]
    pub kind: WsKind,
    #[serde(default)]
    pub content: String,
}

/// A saved WebSocket request. `slug` is the file stem (identity), set on load.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WsRequest {
    #[serde(skip)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subprotocols: Vec<String>,
    #[serde(default, skip_serializing_if = "Auth::is_inherit")]
    pub auth: Auth,
    #[serde(default, skip_serializing_if = "WsSettings::is_default")]
    pub settings: WsSettings,
    #[serde(default, rename = "header", skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KvEntry>,
    #[serde(default, rename = "message", skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<WsMessageTemplate>,
}

impl WsRequest {
    /// Create a minimal websocket request with a slug, name, and url.
    pub fn new(slug: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            name: name.into(),
            url: url.into(),
            ..Self::default()
        }
    }
}
