//! Filesystem layout rules: file names, schema tags, slugging, and child ordering.

use std::collections::{HashMap, HashSet};

use crate::model::Node;

/// Workspace manifest file name.
pub const MANIFEST_FILE: &str = "sasin.toml";
/// Per-folder metadata file name.
pub const FOLDER_FILE: &str = "folder.toml";
/// Environments subdirectory.
pub const ENV_DIR: &str = "environments";
/// Globals file (within [`ENV_DIR`]).
pub const GLOBALS_FILE: &str = "globals.toml";
/// Derived binary cache directory (gitignored).
pub const CACHE_DIR: &str = ".sasin-cache";

/// HTTP request file suffix.
pub const REQUEST_SUFFIX: &str = ".req.toml";
/// WebSocket request file suffix.
pub const WS_SUFFIX: &str = ".ws.toml";

/// Schema discriminators written as the first line of each file.
pub const SCHEMA_WORKSPACE: &str = "sasin/workspace@1";
pub const SCHEMA_FOLDER: &str = "sasin/folder@1";
pub const SCHEMA_REQUEST: &str = "sasin/request@1";
pub const SCHEMA_WEBSOCKET: &str = "sasin/websocket@1";
pub const SCHEMA_ENVIRONMENT: &str = "sasin/environment@1";

/// Classification of a directory entry during load.
pub enum EntryKind {
    /// `*.req.toml` → HTTP request; carries the slug (stem without suffix).
    Http(String),
    /// `*.ws.toml` → WebSocket request; carries the slug.
    Ws(String),
    /// A name that is structural/derived and must be skipped at the tree level.
    Skip,
    /// Anything else (unknown file): ignored.
    Other,
}

/// Classify a file name within a workspace directory.
pub fn classify_file(name: &str) -> EntryKind {
    if name == MANIFEST_FILE || name == FOLDER_FILE || name == ENV_DIR || name == CACHE_DIR {
        return EntryKind::Skip;
    }
    if name.starts_with('.') {
        return EntryKind::Skip;
    }
    if let Some(stem) = name.strip_suffix(REQUEST_SUFFIX) {
        return EntryKind::Http(stem.to_string());
    }
    if let Some(stem) = name.strip_suffix(WS_SUFFIX) {
        return EntryKind::Ws(stem.to_string());
    }
    EntryKind::Other
}

/// Turn a display name into a filesystem-safe slug: lowercase ASCII, `[a-z0-9_-]`, with other
/// runs collapsed to a single `-`. Empty results become `untitled`.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == '_' || ch == '-' {
            Some(ch)
        } else {
            None
        };
        match mapped {
            Some(c) => {
                out.push(c);
                prev_dash = false;
            }
            None => {
                if !prev_dash && !out.is_empty() {
                    out.push('-');
                    prev_dash = true;
                }
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

/// Produce a slug unique within `taken`, appending `-2`, `-3`, … on collision. Inserts the
/// chosen slug into `taken`.
pub fn unique_slug(base: &str, taken: &mut HashSet<String>) -> String {
    let base = slugify(base);
    if taken.insert(base.clone()) {
        return base;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if taken.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

/// Reorder `nodes` by an explicit `order` list of slugs. Listed slugs come first in list order;
/// the rest follow sorted lexically by slug. Stable and deterministic.
pub fn order_nodes(mut nodes: Vec<Node>, order: &[String]) -> Vec<Node> {
    let rank: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    nodes.sort_by(|a, b| {
        let ra = rank.get(a.slug());
        let rb = rank.get(b.slug());
        match (ra, rb) {
            (Some(x), Some(y)) => x.cmp(y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.slug().cmp(b.slug()),
        }
    });
    nodes
}
