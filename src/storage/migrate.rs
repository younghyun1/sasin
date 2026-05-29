//! One-time migration from the legacy binary `.sasin` dataset to a TOML workspace tree.

use std::collections::HashSet;
use std::path::Path;

use crate::model::{Body, Folder, HttpRequest, KvEntry, Node, RawLang, Workspace};
use crate::persist::{Dataset, DatasetFile, Request};
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::layout::unique_slug;
use crate::storage::save::save_workspace;

/// Convert a legacy [`Dataset`] into an in-memory [`Workspace`]: each collection becomes a folder,
/// each request becomes an HTTP request. Slugs are derived from names and de-duplicated.
pub fn workspace_from_dataset(ds: &Dataset, name: &str) -> Workspace {
    let mut top_taken = HashSet::new();
    let mut root = Vec::with_capacity(ds.collections.len());

    for collection in &ds.collections {
        let slug = unique_slug(&collection.name, &mut top_taken);
        let mut child_taken = HashSet::new();
        let children = collection
            .requests
            .iter()
            .map(|r| {
                let rslug = unique_slug(&r.name, &mut child_taken);
                Node::Http(convert_request(r, rslug))
            })
            .collect();

        root.push(Node::Folder(Folder {
            slug,
            name: collection.name.clone(),
            children,
            ..Folder::default()
        }));
    }

    let mut ws = Workspace::default_with_name(name);
    ws.root = root;
    ws
}

fn convert_request(r: &Request, slug: String) -> HttpRequest {
    let headers = r
        .headers
        .iter()
        .map(|h| KvEntry::new(h.name.clone(), h.value.clone()))
        .collect();

    let body = match &r.body {
        Some(text) if !text.trim().is_empty() => {
            let language = if serde_json::from_str::<serde_json::Value>(text).is_ok() {
                RawLang::Json
            } else {
                RawLang::Text
            };
            Body::Raw {
                language,
                text: text.clone(),
            }
        }
        _ => Body::None,
    };

    HttpRequest {
        slug,
        name: r.name.clone(),
        method: r.method.as_str().to_string(),
        url: r.url.clone(),
        headers,
        body,
        ..HttpRequest::default()
    }
}

/// Load a legacy `.sasin` file and write it as a TOML workspace at `dest_dir`. Returns the
/// migrated workspace. The legacy file is left untouched.
pub fn migrate_legacy(legacy: &Path, dest_dir: &Path) -> StorageResult<Workspace> {
    let file = DatasetFile::new(legacy);
    let ds = file
        .load()
        .map_err(|e| StorageError::Migrate(e.to_string()))?;

    let name = dest_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported workspace");

    let ws = workspace_from_dataset(&ds, name);
    save_workspace(dest_dir, &ws)?;
    Ok(ws)
}
