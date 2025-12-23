use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bitcode::{Decode, Encode};

use crate::models::{HeaderEntry, HttpMethod};

/// A stable id for request templates in a dataset.
pub type DatasetId = u64;

/// One saved request template ("tab" in Postman terms).
#[derive(Debug, Clone, Encode, Decode)]
pub struct Request {
    pub id: DatasetId,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub body: Option<String>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Collection {
    pub id: DatasetId,
    pub name: String,
    pub requests: Vec<Request>,
}

/// Stored payload schema (versioned).
#[derive(Debug, Clone, Encode, Decode)]
pub struct Dataset {
    pub version: u32,
    pub collections: Vec<Collection>,
    /// Arbitrary user metadata (e.g. environment, notes).
    pub meta: BTreeMap<String, String>,
}

impl Default for Dataset {
    fn default() -> Self {
        Self {
            version: DatasetFile::FORMAT_VERSION,
            collections: Vec::new(),
            meta: BTreeMap::new(),
        }
    }
}

impl Dataset {
    pub fn upsert(&mut self, mut template: Request) {
        let now = now_unix_ms();
        if template.created_at_unix_ms == 0 {
            template.created_at_unix_ms = now;
        }
        template.updated_at_unix_ms = now;

        for collection in &mut self.collections {
            if let Some(existing) = collection.requests.iter_mut().find(|t| t.id == template.id) {
                *existing = template;
                return;
            }
        }

        // If the request is not found, we need to decide where to put it.
        // For now, let's assume the first collection is the default.
        if let Some(collection) = self.collections.first_mut() {
            collection.requests.push(template);
        } else {
            // Or create a new collection if none exist
            let new_collection = Collection {
                id: self.next_id(),
                name: "My Collection".to_string(),
                requests: vec![template],
            };
            self.collections.push(new_collection);
        }
    }

    pub fn remove(&mut self, id: DatasetId) -> bool {
        for collection in &mut self.collections {
            let before = collection.requests.len();
            collection.requests.retain(|t| t.id != id);
            if before != collection.requests.len() {
                return true;
            }
        }
        false
    }

    pub fn next_id(&self) -> DatasetId {
        let max_request_id = self
            .collections
            .iter()
            .flat_map(|c| c.requests.iter())
            .map(|r| r.id)
            .max()
            .unwrap_or(0);

        let max_collection_id = self.collections.iter().map(|c| c.id).max().unwrap_or(0);

        max_request_id.max(max_collection_id).saturating_add(1)
    }
}

impl Request {
    pub fn new(
        id: DatasetId,
        name: impl Into<String>,
        method: HttpMethod,
        url: impl Into<String>,
    ) -> Self {
        let now = now_unix_ms();
        Self {
            id,
            name: name.into(),
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
        }
    }
}

/// File wrapper and persistence API.
///
/// On disk format:
/// - magic: b"SASINDS1" (8 bytes)
/// - u32: format version (LE)
/// - u32: zstd level (LE) (for informational/debug; decoder ignores and just decompresses)
/// - bytes: zstd-compressed bitcode payload of `Dataset`
///
/// Notes:
/// - `Dataset.version` is the schema version for the payload.
/// - The outer header version is the file container version.
pub struct DatasetFile {
    path: PathBuf,
}

impl DatasetFile {
    pub const MAGIC: [u8; 8] = *b"SASINDS1";
    pub const FORMAT_VERSION: u32 = 2; // Incremented version

    /// Default zstd compression level: reasonably fast and small.
    pub const DEFAULT_ZSTD_LEVEL: i32 = 3;

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create (or load) a dataset file.
    ///
    /// - If the file exists: read & decode it.
    /// - If it does not: returns an empty dataset (caller can save later).
    pub fn load_or_default(&self) -> PersistResult<Dataset> {
        if !self.path.exists() {
            return Ok(Dataset::default());
        }
        self.load()
    }

    /// Read and decode the dataset file from disk.
    pub fn load(&self) -> PersistResult<Dataset> {
        let bytes = fs::read(&self.path).map_err(|e| PersistError::Io(self.path.clone(), e))?;
        decode_dataset_file(&bytes)
            .map_err(|e| e.with_path(self.path.clone()))
            .and_then(|(dataset, version)| {
                if version < Self::FORMAT_VERSION {
                    // Here you would implement migration logic.
                    // For now, we'll just wrap the old templates in a collection.
                    let migrated_dataset = Dataset {
                        version: Self::FORMAT_VERSION,
                        ..dataset
                    };
                    Ok(migrated_dataset)
                } else {
                    Ok(dataset)
                }
            })
    }

    /// Save the dataset to disk (atomic write).
    pub fn save(&self, dataset: &Dataset) -> PersistResult<()> {
        let bytes = encode_dataset_file(dataset, Self::DEFAULT_ZSTD_LEVEL)?;
        atomic_write(&self.path, &bytes).map_err(|e| PersistError::Io(self.path.clone(), e))
    }
}

pub type PersistResult<T> = Result<T, PersistError>;

#[derive(Debug)]
pub enum PersistError {
    Io(PathBuf, io::Error),
    InvalidFormat {
        path: Option<PathBuf>,
        message: String,
    },
    Corrupt {
        path: Option<PathBuf>,
        message: String,
    },
    UnsupportedVersion {
        path: Option<PathBuf>,
        version: u32,
    },
    Encode(String),
    Decode(String),
    Compress(String),
    Decompress(String),
}

impl PersistError {
    fn with_path(mut self, path: PathBuf) -> Self {
        match &mut self {
            PersistError::Io(p, _) => *p = path,
            PersistError::InvalidFormat { path: p, .. } => *p = Some(path),
            PersistError::Corrupt { path: p, .. } => *p = Some(path),
            PersistError::UnsupportedVersion { path: p, .. } => *p = Some(path),
            PersistError::Encode(_) => {}
            PersistError::Decode(_) => {}
            PersistError::Compress(_) => {}
            PersistError::Decompress(_) => {}
        }
        self
    }
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistError::Io(path, e) => write!(f, "I/O error at {}: {e}", path.display()),
            PersistError::InvalidFormat { path, message } => {
                if let Some(p) = path {
                    write!(f, "Invalid dataset format at {}: {message}", p.display())
                } else {
                    write!(f, "Invalid dataset format: {message}")
                }
            }
            PersistError::Corrupt { path, message } => {
                if let Some(p) = path {
                    write!(f, "Corrupt dataset at {}: {message}", p.display())
                } else {
                    write!(f, "Corrupt dataset: {message}")
                }
            }
            PersistError::UnsupportedVersion { path, version } => {
                if let Some(p) = path {
                    write!(
                        f,
                        "Unsupported dataset file version {version} at {}",
                        p.display()
                    )
                } else {
                    write!(f, "Unsupported dataset file version {version}")
                }
            }
            PersistError::Encode(msg) => write!(f, "Encode error: {msg}"),
            PersistError::Decode(msg) => write!(f, "Decode error: {msg}"),
            PersistError::Compress(msg) => write!(f, "Compress error: {msg}"),
            PersistError::Decompress(msg) => write!(f, "Decompress error: {msg}"),
        }
    }
}

impl std::error::Error for PersistError {}

fn encode_dataset_file(dataset: &Dataset, zstd_level: i32) -> PersistResult<Vec<u8>> {
    let payload = bitcode::encode(dataset);

    let compressed = zstd::stream::encode_all(payload.as_slice(), zstd_level)
        .map_err(|e| PersistError::Compress(e.to_string()))?;

    let mut out = Vec::with_capacity(8 + 4 + 4 + compressed.len());
    out.extend_from_slice(&DatasetFile::MAGIC);
    out.extend_from_slice(&DatasetFile::FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(zstd_level as i32).to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn decode_dataset_file(bytes: &[u8]) -> PersistResult<(Dataset, u32)> {
    if bytes.len() < 16 {
        return Err(PersistError::InvalidFormat {
            path: None,
            message: "file too small".to_string(),
        });
    }

    let magic = &bytes[..8];
    if magic != DatasetFile::MAGIC {
        return Err(PersistError::InvalidFormat {
            path: None,
            message: "bad magic".to_string(),
        });
    }

    let ver = u32::from_le_bytes(bytes[8..12].try_into().unwrap());

    let _zstd_level = i32::from_le_bytes(bytes[12..16].try_into().unwrap());

    let compressed = &bytes[16..];
    let decompressed = zstd::stream::decode_all(compressed)
        .map_err(|e| PersistError::Decompress(e.to_string()))?;

    if ver == 1 {
        // Old format, decode as such
        #[derive(Debug, Clone, Encode, Decode)]
        struct OldDataset {
            pub version: u32,
            pub templates: Vec<Request>,
            pub meta: BTreeMap<String, String>,
        }
        let old_dataset: OldDataset =
            bitcode::decode(&decompressed).map_err(|e| PersistError::Decode(e.to_string()))?;
        let new_dataset = Dataset {
            version: 2,
            collections: vec![Collection {
                id: 1, // Or generate a new one
                name: "My Collection".to_string(),
                requests: old_dataset.templates,
            }],
            meta: old_dataset.meta,
        };
        return Ok((new_dataset, ver));
    }

    let dataset: Dataset =
        bitcode::decode(&decompressed).map_err(|e| PersistError::Decode(e.to_string()))?;

    Ok((dataset, ver))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut tmp = path.to_path_buf();
    let tmp_name = format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("dataset")
    );
    tmp.set_file_name(tmp_name);

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(tmp, path)?;
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
