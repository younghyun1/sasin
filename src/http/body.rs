//! Encode a [`Body`] onto a reqwest request builder. File-backed bodies (binary, form-data files)
//! are read relative to the workspace directory.

use std::path::Path;

use reqwest::RequestBuilder;
use reqwest::header::CONTENT_TYPE;

use crate::model::{Body, FormKind, RawLang};

/// Apply `body` to `rb`. Reads referenced files relative to `base_dir`. `user_content_type` is
/// true when the request already carries an explicit `Content-Type` header, in which case the raw
/// body does not add its own (avoids a duplicate header).
pub async fn apply_body(
    rb: RequestBuilder,
    body: &Body,
    base_dir: &Path,
    user_content_type: bool,
) -> Result<RequestBuilder, String> {
    match body {
        Body::None => Ok(rb),
        Body::Raw { language, text } => {
            let rb = if user_content_type {
                rb
            } else {
                rb.header(CONTENT_TYPE, raw_content_type(*language))
            };
            Ok(rb.body(text.clone()))
        }
        Body::UrlEncoded { fields } => {
            let pairs: Vec<(String, String)> = fields
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| (f.key.clone(), f.value.clone()))
                .collect();
            Ok(rb.form(&pairs))
        }
        Body::FormData { parts } => {
            let mut form = reqwest::multipart::Form::new();
            for part in parts {
                if !part.enabled || part.key.trim().is_empty() {
                    continue;
                }
                match part.kind {
                    FormKind::Text => {
                        form = form.text(part.key.clone(), part.value.clone());
                    }
                    FormKind::File => {
                        let path = base_dir.join(&part.src);
                        let data = tokio::fs::read(&path)
                            .await
                            .map_err(|e| format!("form-data file `{}`: {e}", path.display()))?;
                        let file_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let item = reqwest::multipart::Part::bytes(data).file_name(file_name);
                        form = form.part(part.key.clone(), item);
                    }
                }
            }
            Ok(rb.multipart(form))
        }
        Body::Binary { file } => {
            let path = base_dir.join(file);
            let data = tokio::fs::read(&path)
                .await
                .map_err(|e| format!("binary body file `{}`: {e}", path.display()))?;
            Ok(rb.body(data))
        }
        Body::GraphQl { query, variables } => {
            let vars: serde_json::Value = if variables.trim().is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_str(variables)
                    .map_err(|e| format!("GraphQL variables JSON: {e}"))?
            };
            let payload = serde_json::json!({ "query": query, "variables": vars });
            Ok(rb.json(&payload))
        }
    }
}

fn raw_content_type(language: RawLang) -> &'static str {
    match language {
        RawLang::Json => "application/json",
        RawLang::Xml => "application/xml",
        RawLang::Html => "text/html; charset=utf-8",
        RawLang::Javascript => "application/javascript",
        RawLang::Text => "text/plain; charset=utf-8",
    }
}
