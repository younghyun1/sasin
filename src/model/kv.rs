//! Key/value entries shared by query params, headers, and url-encoded bodies.

use serde::{Deserialize, Serialize};

use crate::model::defaults::default_true;

/// A single toggleable key/value pair.
///
/// Used for query parameters (`[[param]]`), request headers (`[[header]]`), and
/// `x-www-form-urlencoded` body fields (`[[body.urlencoded]]`). `enabled = false`
/// keeps the row in the file (so it round-trips) but excludes it when sending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvEntry {
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl KvEntry {
    /// Create an enabled entry.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}
