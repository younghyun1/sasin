//! Git-native storage: map the [`crate::model`] workspace to and from a TOML directory tree,
//! plus a derived, gitignored binary cache. See `docs/planning/02-storage-format.md`.

pub mod cache;
pub mod error;
pub mod layout;
pub mod load;
pub mod migrate;
pub mod save;

pub(crate) mod io_util;

#[cfg(test)]
mod tests;

pub use cache::{
    HistoryCache, HistoryRecord, IndexCache, IndexEntry, KIND_FOLDER, KIND_HTTP, KIND_WS,
    build_index, ensure_gitignore, read_cookies, read_history, read_index, write_cookies,
    write_history, write_index,
};
pub use error::{StorageError, StorageResult};
pub use load::load_workspace;
pub use migrate::{migrate_legacy, workspace_from_dataset};
pub use save::{delete_node, save_nodes, save_workspace, write_manifest};
