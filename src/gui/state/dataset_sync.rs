use crate::models::{HeaderEntry, HttpMethod};
use crate::persist::dataset::Collection;
use crate::persist::{Dataset, DatasetId, Request};

#[derive(Debug, Clone)]
pub enum DatasetUiState {
    Loading { path: std::path::PathBuf },
    Ready { path: std::path::PathBuf },
    Error { message: String },
}

/// A snapshot of the current request editor UI state.
///
/// This is the “draft” that can be applied to a selected `Request`.
#[derive(Debug, Clone, Default)]
pub struct EditorDraft {
    pub method: HttpMethod,
    pub url: String,
    /// Raw headers editor text (`Name: Value` per line).
    pub headers_text: String,
    /// Raw request body editor text.
    pub body_text: String,
    /// Optional name field for the request.
    pub request_name: String,
}

impl EditorDraft {
    pub fn body_option(&self) -> Option<String> {
        let t = self.body_text.trim();
        if t.is_empty() {
            None
        } else {
            Some(self.body_text.clone())
        }
    }
}

/// Ensure there is at least one request in the dataset.
///
/// Returns `true` if a request was added.
pub fn ensure_default_request_exists(dataset: &mut Dataset) -> bool {
    if dataset.collections.is_empty() {
        let next_id = dataset.next_id();
        let mut new_collection = Collection {
            id: next_id,
            name: "My Collection".to_string(),
            requests: Vec::new(),
        };
        new_collection.requests.push(Request::new(
            dataset.next_id(),
            "Default",
            HttpMethod::Get,
            "https://example.com",
        ));
        dataset.collections.push(new_collection);
        return true;
    }

    if dataset.collections.iter().all(|c| c.requests.is_empty()) {
        let next_id = dataset.next_id();
        if let Some(collection) = dataset.collections.first_mut() {
            collection.requests.push(Request::new(
                next_id,
                "Default",
                HttpMethod::Get,
                "https://example.com",
            ));
            return true;
        }
    }

    false
}

/// Load a request into an editor draft.
///
/// Returns `None` if the request is not found.
pub fn load_request_into_editor(dataset: &Dataset, id: DatasetId) -> Option<EditorDraft> {
    for collection in &dataset.collections {
        if let Some(t) = collection.requests.iter().find(|t| t.id == id) {
            return Some(EditorDraft {
                method: t.method,
                url: t.url.clone(),
                headers_text: headers_to_text(&t.headers),
                body_text: t.body.clone().unwrap_or_default(),
                request_name: t.name.clone(),
            });
        }
    }
    None
}

/// Apply the current editor draft into the selected request *immediately*.
///
/// This is the core “immediate mutation” feature:
/// - If `selected_request` exists, we update that request in-place from the editor draft.
/// - If parsing headers fails, we return an error and do not mutate.
/// - If the request name is empty, we keep the existing name (so you can edit request fields
///   without being forced to rename).
///
/// Returns:
/// - `Ok(true)` if a request was updated
/// - `Ok(false)` if no request was selected or found
/// - `Err(msg)` if headers parsing failed
pub fn apply_editor_to_selected_request(
    dataset: &mut Dataset,
    selected_request: Option<DatasetId>,
    draft: &EditorDraft,
) -> Result<bool, String> {
    let Some(id) = selected_request else {
        return Ok(false);
    };

    let mut existing_request = None;
    for collection in &dataset.collections {
        if let Some(request) = collection.requests.iter().find(|t: &&Request| t.id == id) {
            existing_request = Some(request.clone());
            break;
        }
    }

    let Some(existing) = existing_request else {
        return Ok(false);
    };

    let headers = parse_headers(&draft.headers_text)?;

    let mut t = existing;

    let name = draft.request_name.trim();
    if !name.is_empty() {
        t.name = name.to_string();
    }

    t.method = draft.method;
    t.url = draft.url.clone();
    t.headers = headers;
    t.body = draft.body_option();

    dataset.upsert(t);
    Ok(true)
}

/// Parse header editor text into `Vec<HeaderEntry>`.
///
/// Format:
/// - one header per line: `Name: Value`
/// - blank lines are ignored
/// - lines starting with `#` are treated as comments
pub fn parse_headers(raw: &str) -> Result<Vec<HeaderEntry>, String> {
    let mut out = Vec::new();

    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(format!(
                "Invalid header on line {} (expected `Name: Value`): {line}",
                line_no + 1
            ));
        };

        let name = name.trim();
        let value = value.trim();

        if name.is_empty() {
            return Err(format!("Empty header name on line {}", line_no + 1));
        }

        out.push(HeaderEntry {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    Ok(out)
}

/// Format headers as `Name: Value` lines.
pub fn headers_to_text(headers: &[HeaderEntry]) -> String {
    let mut s = String::new();
    for h in headers {
        let name = h.name.trim();
        if name.is_empty() {
            continue;
        }
        s.push_str(name);
        s.push_str(": ");
        s.push_str(h.value.trim());
        s.push('\n');
    }
    s
}
