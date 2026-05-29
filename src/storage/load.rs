//! Read a TOML directory tree into an in-memory [`Workspace`].
//!
//! The `schema` line in each file is an unknown serde field and is ignored. Child order comes
//! from `order` lists; after sorting, the (now redundant) `order` field is cleared so the
//! in-memory model encodes order purely via `Vec` position — keeping `load(save(ws)) == ws`.

use std::fs;
use std::path::Path;

use crate::model::{
    Environment, Folder, HttpRequest, Node, Variable, Workspace, WorkspaceManifest, WsRequest,
};
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::layout::{
    CACHE_DIR, ENV_DIR, EntryKind, FOLDER_FILE, GLOBALS_FILE, MANIFEST_FILE, classify_file,
    order_nodes,
};

/// Load a workspace rooted at `dir`. Missing manifest/environments are treated as empty defaults.
pub fn load_workspace(dir: &Path) -> StorageResult<Workspace> {
    let manifest = read_manifest(dir)?;
    let (environments, globals) = read_environments(dir)?;
    let root = order_nodes(read_dir_nodes(dir)?, &manifest.order);

    // Storage mirrors the manifest faithfully — no derived values are injected. The GUI falls
    // back to the directory name for display when `name` is empty.
    Ok(Workspace {
        name: manifest.name,
        defaults: manifest.defaults,
        root,
        environments,
        globals,
    })
}

fn read_to_string(path: &Path) -> StorageResult<String> {
    fs::read_to_string(path).map_err(|e| StorageError::io(path, e))
}

fn read_manifest(dir: &Path) -> StorageResult<WorkspaceManifest> {
    let path = dir.join(MANIFEST_FILE);
    if !path.exists() {
        return Ok(WorkspaceManifest::default());
    }
    let text = read_to_string(&path)?;
    toml::from_str(&text).map_err(|e| StorageError::TomlDecode(path, e.to_string()))
}

fn read_environments(dir: &Path) -> StorageResult<(Vec<Environment>, Vec<Variable>)> {
    let env_dir = dir.join(ENV_DIR);
    let mut environments = Vec::new();
    let mut globals = Vec::new();
    if !env_dir.is_dir() {
        return Ok((environments, globals));
    }

    for entry in fs::read_dir(&env_dir).map_err(|e| StorageError::io(&env_dir, e))? {
        let entry = entry.map_err(|e| StorageError::io(&env_dir, e))?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with('.') || !fname.ends_with(".toml") {
            continue;
        }
        let path = entry.path();
        let text = read_to_string(&path)?;
        let mut env: Environment = toml::from_str(&text)
            .map_err(|e| StorageError::TomlDecode(path.clone(), e.to_string()))?;
        let stem = fname.strip_suffix(".toml").unwrap_or(&fname);
        env.slug = stem.to_string();
        if fname == GLOBALS_FILE {
            globals = env.variables;
        } else {
            environments.push(env);
        }
    }

    environments.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok((environments, globals))
}

fn read_dir_nodes(dir: &Path) -> StorageResult<Vec<Node>> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| StorageError::io(dir, e))? {
        let entry = entry.map_err(|e| StorageError::io(dir, e))?;
        let fname = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type().map_err(|e| StorageError::io(dir, e))?;
        let path = entry.path();

        if file_type.is_dir() {
            if fname.starts_with('.')
                || fname == ENV_DIR
                || fname == CACHE_DIR
                || fname.ends_with(".examples")
            {
                continue;
            }
            nodes.push(Node::Folder(read_folder(&path, &fname)?));
        } else {
            match classify_file(&fname) {
                EntryKind::Http(slug) => nodes.push(Node::Http(read_request(&path, &slug)?)),
                EntryKind::Ws(slug) => nodes.push(Node::Ws(read_ws(&path, &slug)?)),
                EntryKind::Skip | EntryKind::Other => {}
            }
        }
    }
    Ok(nodes)
}

fn read_folder(path: &Path, slug: &str) -> StorageResult<Folder> {
    let meta_path = path.join(FOLDER_FILE);
    let mut folder: Folder = if meta_path.exists() {
        let text = read_to_string(&meta_path)?;
        toml::from_str(&text).map_err(|e| StorageError::TomlDecode(meta_path, e.to_string()))?
    } else {
        Folder::default()
    };

    folder.slug = slug.to_string();
    let children = order_nodes(read_dir_nodes(path)?, &folder.order);
    folder.order = Vec::new(); // order now encoded by children position
    folder.children = children;
    Ok(folder)
}

fn read_request(path: &Path, slug: &str) -> StorageResult<HttpRequest> {
    let text = read_to_string(path)?;
    let mut req: HttpRequest = toml::from_str(&text)
        .map_err(|e| StorageError::TomlDecode(path.to_path_buf(), e.to_string()))?;
    req.slug = slug.to_string();
    Ok(req)
}

fn read_ws(path: &Path, slug: &str) -> StorageResult<WsRequest> {
    let text = read_to_string(path)?;
    let mut ws: WsRequest = toml::from_str(&text)
        .map_err(|e| StorageError::TomlDecode(path.to_path_buf(), e.to_string()))?;
    ws.slug = slug.to_string();
    Ok(ws)
}
