//! Pre-request and test scripts attached to a request or folder.
//!
//! Stored even before the scripting engine (phase P6) is wired, so the schema is stable.
//! Empty strings mean "no script" and are omitted from serialized TOML.

use serde::{Deserialize, Serialize};

/// JavaScript run around a request: `pre_request` before sending, `test` after the response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scripts {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pre_request: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub test: String,
}

impl Scripts {
    /// True when neither script has content (used to omit `[scripts]` from TOML).
    pub fn is_empty(&self) -> bool {
        self.pre_request.is_empty() && self.test.is_empty()
    }
}
