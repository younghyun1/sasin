/// Dataset <-> editor synchronization helpers.
///
/// Goal:
/// - When a template is selected, any editor change (method/url/headers/body/name)
///   immediately mutates that template in-memory.
/// - The GUI layer can then autosave (debounced) by simply persisting the dataset.
///
/// This module keeps `gui/app.rs` lean by centralizing:
/// - parsing/formatting headers text
/// - applying editor state to the selected template
/// - loading a template into the editor UI state
///
/// NOTE: Keep this module free of any Iced types so it is testable and reusable.
use crate::models::{HeaderEntry, HttpMethod};
use crate::persist::{Dataset, DatasetId, RequestTemplate};

#[derive(Debug, Clone)]
pub enum DatasetUiState {
    Loading { path: std::path::PathBuf },
    Ready { path: std::path::PathBuf },
    Error { message: String },
}

/// A snapshot of the current request editor UI state.
///
/// This is the “draft” that can be applied to a selected `RequestTemplate`.
#[derive(Debug, Clone, Default)]
pub struct EditorDraft {
    pub method: HttpMethod,
    pub url: String,
    /// Raw headers editor text (`Name: Value` per line).
    pub headers_text: String,
    /// Raw request body editor text.
    pub body_text: String,
    /// Optional name field for the template.
    pub template_name: String,
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

/// Ensure there is at least one template in the dataset.
///
/// Returns `true` if a template was added.
pub fn ensure_default_template_exists(dataset: &mut Dataset) -> bool {
    if !dataset.templates.is_empty() {
        return false;
    }

    let id = dataset.next_id();
    dataset.templates.push(RequestTemplate::new(
        id,
        "Default",
        HttpMethod::Get,
        "https://example.com",
    ));
    true
}

/// Load a template into an editor draft.
///
/// Returns `None` if the template is not found.
pub fn load_template_into_editor(dataset: &Dataset, id: DatasetId) -> Option<EditorDraft> {
    let t = dataset.templates.iter().find(|t| t.id == id)?;

    Some(EditorDraft {
        method: t.method,
        url: t.url.clone(),
        headers_text: headers_to_text(&t.headers),
        body_text: t.body.clone().unwrap_or_default(),
        template_name: t.name.clone(),
    })
}

/// Apply the current editor draft into the selected template *immediately*.
///
/// This is the core “immediate mutation” feature:
/// - If `selected_template` exists, we update that template in-place from the editor draft.
/// - If parsing headers fails, we return an error and do not mutate.
/// - If the template name is empty, we keep the existing name (so you can edit request fields
///   without being forced to rename).
///
/// Returns:
/// - `Ok(true)` if a template was updated
/// - `Ok(false)` if no template was selected or found
/// - `Err(msg)` if headers parsing failed
pub fn apply_editor_to_selected_template(
    dataset: &mut Dataset,
    selected_template: Option<DatasetId>,
    draft: &EditorDraft,
) -> Result<bool, String> {
    let Some(id) = selected_template else {
        return Ok(false);
    };

    let Some(existing) = dataset.templates.iter().find(|t| t.id == id).cloned() else {
        return Ok(false);
    };

    let headers = parse_headers(&draft.headers_text)?;

    let mut t = existing;

    let name = draft.template_name.trim();
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
