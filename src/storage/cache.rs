//! Derived binary cache, gitignored and always rebuildable from the TOML tree.
//!
//! Two homes:
//! - A per-workspace cache directory under the app state dir (keyed by a hash of the workspace
//!   path), holding the flattened index, session, and history.
//! - A `.gitignore` written into the workspace so `.sasin-cache/` is never committed.
//!
//! Encoding is `bitcode` + `zstd` (compact + fast); the format is private and disposable.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use bitcode::{Decode, Encode};

use crate::model::{Node, Workspace};
use crate::persist::app_state_dir;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::io_util::write_atomic;
use crate::storage::layout::CACHE_DIR;

/// Node kind tag in the flattened index.
pub const KIND_FOLDER: u8 = 0;
pub const KIND_HTTP: u8 = 1;
pub const KIND_WS: u8 = 2;

/// One entry of the flattened tree index.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct IndexEntry {
    /// Workspace-relative path without extension, e.g. `Users/list`.
    pub path: String,
    pub kind: u8,
    pub name: String,
    /// HTTP method for request nodes; empty otherwise.
    pub method: String,
    /// Request URL (http and ws nodes); empty for folders. Searchable.
    pub url: String,
}

/// Flattened tree index for fast sidebar render + search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode)]
pub struct IndexCache {
    pub workspace_path: String,
    pub entries: Vec<IndexEntry>,
}

/// One persisted request-history record.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HistoryRecord {
    pub method: String,
    pub url: String,
    pub at_unix_ms: u64,
}

/// Persisted request history (fixes the previously in-memory-only history).
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode)]
pub struct HistoryCache {
    pub records: Vec<HistoryRecord>,
}

/// Resolve the per-workspace cache directory under the app state dir.
pub fn cache_root(workspace_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    workspace_path.to_string_lossy().hash(&mut hasher);
    app_state_dir()
        .join("workspace-cache")
        .join(format!("{:016x}", hasher.finish()))
}

/// Build the flattened index from an in-memory workspace.
pub fn build_index(workspace_path: &Path, ws: &Workspace) -> IndexCache {
    let mut entries = Vec::new();
    collect_index(&ws.root, "", &mut entries);
    IndexCache {
        workspace_path: workspace_path.display().to_string(),
        entries,
    }
}

fn collect_index(nodes: &[Node], prefix: &str, out: &mut Vec<IndexEntry>) {
    for node in nodes {
        let path = if prefix.is_empty() {
            node.slug().to_string()
        } else {
            format!("{prefix}/{}", node.slug())
        };
        match node {
            Node::Folder(f) => {
                out.push(IndexEntry {
                    path: path.clone(),
                    kind: KIND_FOLDER,
                    name: f.name.clone(),
                    method: String::new(),
                    url: String::new(),
                });
                collect_index(&f.children, &path, out);
            }
            Node::Http(r) => out.push(IndexEntry {
                path,
                kind: KIND_HTTP,
                name: r.name.clone(),
                method: r.method.clone(),
                url: r.url.clone(),
            }),
            Node::Ws(w) => out.push(IndexEntry {
                path,
                kind: KIND_WS,
                name: w.name.clone(),
                method: String::new(),
                url: w.url.clone(),
            }),
        }
    }
}

/// Write a bitcode+zstd cache value atomically.
pub fn write_cache<T: Encode>(path: &Path, value: &T) -> StorageResult<()> {
    let payload = bitcode::encode(value);
    let compressed = zstd::stream::encode_all(payload.as_slice(), 3)
        .map_err(|e| StorageError::Cache(e.to_string()))?;
    write_atomic(path, &compressed)
}

/// Read a bitcode+zstd cache value. Returns `None` on any read/decode failure (cache is disposable).
pub fn read_cache<T: for<'a> Decode<'a>>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    let decompressed = zstd::stream::decode_all(bytes.as_slice()).ok()?;
    bitcode::decode(&decompressed).ok()
}

/// Persist the index for a workspace.
pub fn write_index(workspace_path: &Path, index: &IndexCache) -> StorageResult<()> {
    let root = cache_root(workspace_path);
    write_cache(&root.join("index.bc.zst"), index)
}

/// Read the index for a workspace, if present and valid.
pub fn read_index(workspace_path: &Path) -> Option<IndexCache> {
    read_cache(&cache_root(workspace_path).join("index.bc.zst"))
}

/// Persist request history for a workspace.
pub fn write_history(workspace_path: &Path, history: &HistoryCache) -> StorageResult<()> {
    let root = cache_root(workspace_path);
    write_cache(&root.join("history.bc.zst"), history)
}

/// Read request history for a workspace.
pub fn read_history(workspace_path: &Path) -> HistoryCache {
    read_cache(&cache_root(workspace_path).join("history.bc.zst")).unwrap_or_default()
}

/// Ensure the workspace `.gitignore` excludes the derived cache directory.
pub fn ensure_gitignore(workspace_dir: &Path) -> StorageResult<()> {
    let path = workspace_dir.join(".gitignore");
    let entry = format!("{CACHE_DIR}/");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim() == entry || l.trim() == CACHE_DIR)
    {
        return Ok(());
    }
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&entry);
    content.push('\n');
    fs::write(&path, content.as_bytes()).map_err(|e| StorageError::io(&path, e))
}
