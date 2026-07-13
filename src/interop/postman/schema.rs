//! Lenient serde model of the Postman Collection v2.1 JSON. Every field is defaulted so
//! partially-populated exports never fail to parse; unknown fields are ignored.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct Collection {
    pub info: Info,
    pub item: Vec<Item>,
    pub auth: Option<AuthDef>,
    pub variable: Vec<VariableDef>,
    pub event: Vec<EventDef>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct Info {
    pub name: String,
    pub description: Option<Description>,
}

/// Postman descriptions are either a bare string or `{ content, type }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum Description {
    Text(String),
    Object {
        #[serde(default)]
        content: String,
    },
}

impl Description {
    pub fn text(&self) -> &str {
        match self {
            Description::Text(s) => s,
            Description::Object { content } => content,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct Item {
    pub name: String,
    /// Present on folders.
    pub item: Option<Vec<Item>>,
    /// Present on requests.
    pub request: Option<RequestUnion>,
    pub auth: Option<AuthDef>,
    pub event: Vec<EventDef>,
    pub description: Option<Description>,
}

/// A request is either a bare URL string or a full object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum RequestUnion {
    Url(String),
    Object(Box<RequestObj>),
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct RequestObj {
    pub method: Option<String>,
    pub url: Option<UrlUnion>,
    pub header: Vec<HeaderDef>,
    pub body: Option<BodyDef>,
    pub auth: Option<AuthDef>,
    pub description: Option<Description>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum UrlUnion {
    Raw(String),
    Object(UrlObj),
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct UrlObj {
    pub raw: String,
    pub query: Vec<QueryDef>,
    /// Path variables (`:id` segments); unsupported on import (left literal).
    pub variable: Vec<VariableDef>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct HeaderDef {
    pub key: String,
    pub value: String,
    pub disabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct QueryDef {
    pub key: String,
    pub value: Option<String>,
    pub disabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct BodyDef {
    pub mode: Option<String>,
    pub raw: Option<String>,
    pub options: Option<BodyOptions>,
    pub urlencoded: Vec<KvDef>,
    pub formdata: Vec<FormDataDef>,
    pub file: Option<FileDef>,
    pub graphql: Option<GraphqlDef>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct BodyOptions {
    pub raw: Option<RawOptions>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct RawOptions {
    pub language: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct KvDef {
    pub key: String,
    pub value: String,
    pub disabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct FormDataDef {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub value: Option<String>,
    /// A path string, or an array of paths for multi-file fields.
    pub src: Option<serde_json::Value>,
    pub disabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct FileDef {
    pub src: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct GraphqlDef {
    pub query: Option<String>,
    pub variables: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AuthDef {
    #[serde(rename = "type")]
    pub kind: String,
    pub basic: Vec<AuthParam>,
    pub bearer: Vec<AuthParam>,
    pub apikey: Vec<AuthParam>,
    pub oauth2: Vec<AuthParam>,
}

/// v2.1 encodes auth params as `{ key, value, type }` lists; values may be non-strings.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AuthParam {
    pub key: String,
    pub value: serde_json::Value,
}

/// Fetch an auth param by key, stringifying non-string values.
pub(super) fn auth_param(list: &[AuthParam], key: &str) -> Option<String> {
    list.iter().find(|p| p.key == key).map(|p| match &p.value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct VariableDef {
    pub key: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct EventDef {
    pub listen: String,
    pub script: Option<ScriptDef>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct ScriptDef {
    pub exec: ExecUnion,
}

/// Script sources are a single string or a list of lines.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ExecUnion {
    One(String),
    Lines(Vec<String>),
}

impl Default for ExecUnion {
    fn default() -> Self {
        ExecUnion::Lines(Vec::new())
    }
}

impl ExecUnion {
    pub fn joined(&self) -> String {
        match self {
            ExecUnion::One(s) => s.clone(),
            ExecUnion::Lines(lines) => lines.join("\n"),
        }
    }
}
