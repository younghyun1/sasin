//! Serialize an in-memory [`Workspace`] to the TOML directory tree.
//!
//! Each file gets a `schema = "…"` first line (prepended, not a struct field). Folder and
//! manifest `order` lists are regenerated from the current children so they never drift. Writes
//! are atomic. This performs a full write of the model's files; it does not prune files for nodes
//! that were removed (the GUI deletes through an explicit path).

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::model::{Environment, Node, Variable, Workspace};
use crate::storage::cache::ensure_gitignore;
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::io_util::write_atomic;
use crate::storage::layout::{
    ENV_DIR, FOLDER_FILE, GLOBALS_FILE, MANIFEST_FILE, REQUEST_SUFFIX, SCHEMA_ENVIRONMENT,
    SCHEMA_FOLDER, SCHEMA_REQUEST, SCHEMA_WEBSOCKET, SCHEMA_WORKSPACE, WS_SUFFIX, slugify,
};

/// Serialize `value` to TOML with a leading `schema` line.
fn to_toml_with_schema<T: Serialize>(value: &T, schema: &str) -> StorageResult<String> {
    let body =
        toml::to_string_pretty(value).map_err(|e| StorageError::TomlEncode(e.to_string()))?;
    Ok(format!("schema = \"{schema}\"\n{body}"))
}

/// Write the full workspace tree to `dir`, (re)creating the directory and `.gitignore`.
pub fn save_workspace(dir: &Path, ws: &Workspace) -> StorageResult<()> {
    fs::create_dir_all(dir).map_err(|e| StorageError::io(dir, e))?;
    ensure_gitignore(dir)?;
    write_manifest(dir, ws)?;
    save_environments(dir, ws)?;
    save_nodes(dir, &ws.root)
}

/// Write `sasin.toml`.
pub fn write_manifest(dir: &Path, ws: &Workspace) -> StorageResult<()> {
    let s = to_toml_with_schema(&ws.manifest(), SCHEMA_WORKSPACE)?;
    write_atomic(&dir.join(MANIFEST_FILE), s.as_bytes())
}

fn save_environments(dir: &Path, ws: &Workspace) -> StorageResult<()> {
    let env_dir = dir.join(ENV_DIR);
    if !ws.globals.is_empty() {
        write_env_file(&env_dir.join(GLOBALS_FILE), "globals", &ws.globals)?;
    }
    for env in &ws.environments {
        let slug = if env.slug.is_empty() {
            slugify(&env.name)
        } else {
            env.slug.clone()
        };
        write_env_file(
            &env_dir.join(format!("{slug}.toml")),
            &env.name,
            &env.variables,
        )?;
    }
    Ok(())
}

fn write_env_file(path: &Path, name: &str, variables: &[Variable]) -> StorageResult<()> {
    let env = Environment {
        slug: String::new(),
        name: name.to_string(),
        variables: variables.to_vec(),
    };
    let s = to_toml_with_schema(&env, SCHEMA_ENVIRONMENT)?;
    write_atomic(path, s.as_bytes())
}

/// Recursively write nodes into `dir`.
pub fn save_nodes(dir: &Path, nodes: &[Node]) -> StorageResult<()> {
    for node in nodes {
        match node {
            Node::Folder(folder) => {
                let child_dir = dir.join(&folder.slug);
                fs::create_dir_all(&child_dir).map_err(|e| StorageError::io(&child_dir, e))?;

                // Regenerate `order` from the live children so it never drifts.
                let mut meta = folder.clone();
                meta.order = folder
                    .children
                    .iter()
                    .map(|n| n.slug().to_string())
                    .collect();
                let s = to_toml_with_schema(&meta, SCHEMA_FOLDER)?;
                write_atomic(&child_dir.join(FOLDER_FILE), s.as_bytes())?;

                save_nodes(&child_dir, &folder.children)?;
            }
            Node::Http(req) => {
                let s = to_toml_with_schema(req, SCHEMA_REQUEST)?;
                write_atomic(
                    &dir.join(format!("{}{REQUEST_SUFFIX}", req.slug)),
                    s.as_bytes(),
                )?;
            }
            Node::Ws(ws) => {
                let s = to_toml_with_schema(ws, SCHEMA_WEBSOCKET)?;
                write_atomic(&dir.join(format!("{}{WS_SUFFIX}", ws.slug)), s.as_bytes())?;
            }
        }
    }
    Ok(())
}
