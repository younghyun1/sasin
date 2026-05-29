//! Environments and variables.
//!
//! An environment file (`environments/<slug>.toml`) is a named set of variables;
//! `environments/globals.toml` is the lowest-priority scope. Variables interpolate
//! `{{key}}` tokens at send time (phase P4).

use serde::{Deserialize, Serialize};

use crate::model::defaults::default_true;

/// A single environment variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Masked in the UI (value still stored on disk).
    #[serde(default)]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Variable {
    /// Create an enabled, non-secret variable.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            secret: false,
            description: None,
        }
    }
}

/// A named environment (set of variables). `slug` is the file stem, set on load and
/// never serialized into the file body.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Environment {
    #[serde(skip)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, rename = "variable", skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
}
