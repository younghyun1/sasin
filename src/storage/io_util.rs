//! Shared atomic file write: write to a sibling temp file, fsync, then rename over the target.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::storage::error::{StorageError, StorageResult};

/// Atomically write `bytes` to `path`, creating parent directories as needed.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
    }

    let mut tmp = path.to_path_buf();
    let tmp_name = format!(
        ".{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("file")
    );
    tmp.set_file_name(tmp_name);

    {
        let mut f = fs::File::create(&tmp).map_err(|e| StorageError::io(&tmp, e))?;
        f.write_all(bytes).map_err(|e| StorageError::io(&tmp, e))?;
        f.sync_all().map_err(|e| StorageError::io(&tmp, e))?;
    }

    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path).map_err(|e| StorageError::io(path, e))
}
