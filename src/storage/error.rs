//! Error type for the storage layer.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Result alias for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Failures while reading or writing the workspace tree or its cache.
#[derive(Debug)]
pub enum StorageError {
    /// Filesystem I/O error at a path.
    Io(PathBuf, io::Error),
    /// TOML failed to deserialize from a file.
    TomlDecode(PathBuf, String),
    /// A value failed to serialize to TOML.
    TomlEncode(String),
    /// Cache (bitcode/zstd) read/write failure.
    Cache(String),
    /// Legacy dataset migration failure.
    Migrate(String),
    /// Directory nesting exceeded the recursion limit (guards against symlink cycles).
    TooDeep(PathBuf),
}

impl StorageError {
    /// Attach a path to an I/O error from a closure.
    pub fn io(path: &Path, e: io::Error) -> Self {
        StorageError::Io(path.to_path_buf(), e)
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(path, e) => write!(f, "I/O error at {}: {e}", path.display()),
            StorageError::TomlDecode(path, msg) => {
                write!(f, "failed to parse {}: {msg}", path.display())
            }
            StorageError::TomlEncode(msg) => write!(f, "failed to serialize TOML: {msg}"),
            StorageError::Cache(msg) => write!(f, "cache error: {msg}"),
            StorageError::Migrate(msg) => write!(f, "migration error: {msg}"),
            StorageError::TooDeep(path) => {
                write!(f, "directory nesting too deep at {}", path.display())
            }
        }
    }
}

impl std::error::Error for StorageError {}
