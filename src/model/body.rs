//! Request body, modeled after Postman's body modes.
//!
//! Serialized as an internally-tagged TOML table keyed by `mode`:
//! ```toml
//! [body]
//! mode = "raw"
//! language = "json"
//! text = '''{"a":1}'''
//! ```

use serde::{Deserialize, Serialize};

use crate::model::defaults::default_true;
use crate::model::kv::KvEntry;

/// Highlight/content language for a raw body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawLang {
    #[default]
    Json,
    Text,
    Xml,
    Html,
    Javascript,
}

impl RawLang {
    /// All variants, for rendering a picker.
    pub const fn all() -> &'static [RawLang] {
        &[
            RawLang::Json,
            RawLang::Text,
            RawLang::Xml,
            RawLang::Html,
            RawLang::Javascript,
        ]
    }

    /// Human-facing label.
    pub const fn label(self) -> &'static str {
        match self {
            RawLang::Json => "JSON",
            RawLang::Text => "Text",
            RawLang::Xml => "XML",
            RawLang::Html => "HTML",
            RawLang::Javascript => "JavaScript",
        }
    }
}

impl std::fmt::Display for RawLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A `multipart/form-data` part: either an inline text value or a file reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormKind {
    #[default]
    Text,
    File,
}

/// One `multipart/form-data` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormPart {
    pub key: String,
    #[serde(default)]
    pub kind: FormKind,
    /// Inline value when `kind = text`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// Relative file path when `kind = file`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub src: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Request body content.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Body {
    /// No body.
    #[default]
    None,
    /// Raw text in some language (json/text/xml/html/javascript).
    Raw {
        #[serde(default)]
        language: RawLang,
        #[serde(default)]
        text: String,
    },
    /// `application/x-www-form-urlencoded`.
    #[serde(rename = "urlencoded")]
    UrlEncoded {
        #[serde(default, rename = "urlencoded")]
        fields: Vec<KvEntry>,
    },
    /// `multipart/form-data`.
    #[serde(rename = "formdata")]
    FormData {
        #[serde(default, rename = "formdata")]
        parts: Vec<FormPart>,
    },
    /// Raw bytes from a relative file path.
    Binary { file: String },
    /// GraphQL query + JSON variables.
    #[serde(rename = "graphql")]
    GraphQl {
        #[serde(default)]
        query: String,
        #[serde(default)]
        variables: String,
    },
}

impl Body {
    /// True for [`Body::None`] (the default), used to omit `[body]` from TOML.
    pub fn is_none(&self) -> bool {
        matches!(self, Body::None)
    }
}
