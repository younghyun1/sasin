//! HTTP request definition (`*.req.toml`).
//!
//! Field order matters: TOML requires all scalar/inline values before any sub-table or
//! array-of-tables. Hence scalars (`name`, `description`, `method`, `url`) precede the
//! `[[param]]`/`[[header]]` arrays and the `[auth]`/`[body]`/`[settings]`/`[scripts]` tables.
//!
//! Timestamps are intentionally **not** stored — git history is the source of truth, and
//! per-save timestamps would create diff churn and merge conflicts.

use serde::{Deserialize, Serialize};

use crate::model::auth::Auth;
use crate::model::body::Body;
use crate::model::defaults::default_get;
use crate::model::kv::KvEntry;
use crate::model::scripts::Scripts;
use crate::model::settings::Settings;

/// A saved HTTP request. `slug` is the file stem (identity), set on load and never written
/// into the file body. `method` is a free `String` to allow custom verbs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    #[serde(skip)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_get")]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "param", skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<KvEntry>,
    #[serde(default, rename = "header", skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KvEntry>,
    #[serde(default, skip_serializing_if = "Auth::is_inherit")]
    pub auth: Auth,
    #[serde(default, skip_serializing_if = "Body::is_none")]
    pub body: Body,
    #[serde(default, skip_serializing_if = "Settings::is_default")]
    pub settings: Settings,
    #[serde(default, skip_serializing_if = "Scripts::is_empty")]
    pub scripts: Scripts,
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self {
            slug: String::new(),
            name: String::new(),
            description: None,
            method: default_get(),
            url: String::new(),
            params: Vec::new(),
            headers: Vec::new(),
            auth: Auth::default(),
            body: Body::default(),
            settings: Settings::default(),
            scripts: Scripts::default(),
        }
    }
}

impl HttpRequest {
    /// Create a minimal request with a slug, name, method, and url.
    pub fn new(
        slug: impl Into<String>,
        name: impl Into<String>,
        method: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            slug: slug.into(),
            name: name.into(),
            method: method.into(),
            url: url.into(),
            ..Self::default()
        }
    }
}
