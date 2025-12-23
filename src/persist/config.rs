use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use bitcode::{Decode, Encode};

use crate::persist::{Dataset, DatasetFile, PersistError};

/// Persisted application configuration.
///
/// This is intentionally small and stable. It is stored separately from datasets,
/// and is used to remember "last opened dataset" and basic window state.
///
/// Note: `bitcode` does not implement `Encode/Decode` for `PathBuf` out of the box.
/// Store paths as UTF-8 strings and convert at the edges.
#[derive(Debug, Clone, Default, Encode, Decode)]
pub struct AppConfig {
    pub version: u32,

    /// Last dataset file path the user opened/saved (UTF-8).
    pub last_dataset_path: Option<String>,

    /// Autosave dataset/templates (debounced) while editing.
    ///
    /// When decoding legacy configs where this field was not present, we treat
    /// it as enabled by default (see `load_or_default` fallback path below).
    pub autosave_enabled: bool,

    /// Split pane sizes (pixels). These are clamped by the UI at runtime.
    pub layout: Option<LayoutState>,

    /// Optional window state.
    pub window: Option<WindowState>,
}

/// Persisted UI layout state (pixel sizes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct LayoutState {
    /// Width of the left sidebar (templates/history) in pixels.
    pub sidebar_width_px: u32,

    /// Height of the request editor pane in pixels (top pane of main area).
    pub request_height_px: u32,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            sidebar_width_px: 340,
            request_height_px: 420,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1100,
            height: 780,
            maximized: false,
        }
    }
}

/// App config file wrapper and persistence API.
///
/// On disk format:
/// - magic: b"SASINCFG" (8 bytes)
/// - u32: format version (LE)
/// - u32: zstd level (LE) (informational; decoder ignores and just decompresses)
/// - bytes: zstd-compressed bitcode payload of `AppConfig`
pub struct AppConfigFile {
    path: PathBuf,
}

impl AppConfigFile {
    pub const MAGIC: [u8; 8] = *b"SASINCFG";
    pub const FORMAT_VERSION: u32 = 1;

    /// Default zstd compression level for config: fast and small.
    pub const DEFAULT_ZSTD_LEVEL: i32 = 1;

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load config from disk, or return defaults if the file does not exist.
    pub fn load_or_default(&self) -> Result<AppConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(AppConfig {
                version: Self::FORMAT_VERSION,
                autosave_enabled: true,
                layout: Some(LayoutState::default()),
                ..AppConfig::default()
            });
        }

        // Backwards compatibility: if an older config file is present but fails
        // to decode (e.g. due to a schema change), fall back to defaults with
        // autosave enabled rather than failing startup.
        match self.load() {
            Ok(mut cfg) => {
                // Ensure sane defaults for fields that may be missing or invalid.
                if cfg.version == 0 {
                    cfg.version = Self::FORMAT_VERSION;
                }

                // Default autosave to enabled when loading older configs.
                cfg.autosave_enabled = cfg.autosave_enabled || cfg.version < Self::FORMAT_VERSION;

                // Ensure layout defaults exist.
                if cfg.layout.is_none() {
                    cfg.layout = Some(LayoutState::default());
                }

                Ok(cfg)
            }
            Err(_) => Ok(AppConfig {
                version: Self::FORMAT_VERSION,
                autosave_enabled: true,
                layout: Some(LayoutState::default()),
                ..AppConfig::default()
            }),
        }
    }

    /// Load config from disk.
    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        let bytes = fs::read(&self.path).map_err(|e| ConfigError::Io(self.path.clone(), e))?;
        decode_config_file(&bytes).map_err(|e| e.with_path(self.path.clone()))
    }

    /// Save config to disk (atomic write).
    pub fn save(&self, cfg: &AppConfig) -> Result<(), ConfigError> {
        let bytes = encode_config_file(cfg, Self::DEFAULT_ZSTD_LEVEL)?;
        atomic_write(&self.path, &bytes).map_err(|e| ConfigError::Io(self.path.clone(), e))
    }
}

/// Helper for the common startup behavior you requested:
/// - If `cfg.last_dataset_path` exists, try to load it.
/// - Otherwise, create/load the default dataset path.
/// - Ensure there's at least one template (default template).
///
/// Returns:
/// - the dataset path actually used
/// - the loaded dataset
pub fn load_startup_dataset(
    cfg: &AppConfig,
    default_dataset_path: impl Into<PathBuf>,
) -> Result<(PathBuf, Dataset), String> {
    let default_path = default_dataset_path.into();

    let candidate: PathBuf = cfg
        .last_dataset_path
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_path);

    let file = DatasetFile::new(&candidate);
    let mut ds = file.load_or_default().map_err(|e| e.to_string())?;

    // Ensure at least one default template exists.
    if ds.templates.is_empty() {
        ds.templates.push(crate::persist::RequestTemplate::new(
            ds.next_id(),
            "Default",
            crate::models::HttpMethod::Get,
            "https://example.com",
        ));
        file.save(&ds).map_err(|e| e.to_string())?;
    }

    Ok((candidate, ds))
}

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug)]
pub enum ConfigError {
    Io(PathBuf, io::Error),
    InvalidFormat {
        path: Option<PathBuf>,
        message: String,
    },
    UnsupportedVersion {
        path: Option<PathBuf>,
        version: u32,
    },
    Decode(String),
    Compress(String),
    Decompress(String),
}

impl ConfigError {
    fn with_path(mut self, path: PathBuf) -> Self {
        match &mut self {
            ConfigError::Io(p, _) => *p = path,
            ConfigError::InvalidFormat { path: p, .. } => *p = Some(path),
            ConfigError::UnsupportedVersion { path: p, .. } => *p = Some(path),
            ConfigError::Decode(_) => {}
            ConfigError::Compress(_) => {}
            ConfigError::Decompress(_) => {}
        }
        self
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(path, e) => write!(f, "I/O error at {}: {e}", path.display()),
            ConfigError::InvalidFormat { path, message } => {
                if let Some(p) = path {
                    write!(f, "Invalid config format at {}: {message}", p.display())
                } else {
                    write!(f, "Invalid config format: {message}")
                }
            }
            ConfigError::UnsupportedVersion { path, version } => {
                if let Some(p) = path {
                    write!(
                        f,
                        "Unsupported config file version {version} at {}",
                        p.display()
                    )
                } else {
                    write!(f, "Unsupported config file version {version}")
                }
            }
            ConfigError::Decode(msg) => write!(f, "Decode error: {msg}"),
            ConfigError::Compress(msg) => write!(f, "Compress error: {msg}"),
            ConfigError::Decompress(msg) => write!(f, "Decompress error: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for String {
    fn from(e: ConfigError) -> Self {
        e.to_string()
    }
}

/// Convert dataset PersistError to a string without leaking internals.
/// (Keeps GUI-facing APIs simple.)
pub fn persist_error_to_string(e: PersistError) -> String {
    e.to_string()
}

/// Encode config file bytes.
fn encode_config_file(cfg: &AppConfig, zstd_level: i32) -> ConfigResult<Vec<u8>> {
    let payload = bitcode::encode(cfg);

    let compressed = zstd::stream::encode_all(payload.as_slice(), zstd_level)
        .map_err(|e| ConfigError::Compress(e.to_string()))?;

    let mut out = Vec::with_capacity(8 + 4 + 4 + compressed.len());
    out.extend_from_slice(&AppConfigFile::MAGIC);
    out.extend_from_slice(&AppConfigFile::FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(zstd_level as i32).to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Decode config file bytes.
fn decode_config_file(bytes: &[u8]) -> ConfigResult<AppConfig> {
    if bytes.len() < 16 {
        return Err(ConfigError::InvalidFormat {
            path: None,
            message: "file too small".to_string(),
        });
    }

    let magic = &bytes[..8];
    if magic != AppConfigFile::MAGIC {
        return Err(ConfigError::InvalidFormat {
            path: None,
            message: "bad magic".to_string(),
        });
    }

    let ver = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if ver != AppConfigFile::FORMAT_VERSION {
        return Err(ConfigError::UnsupportedVersion {
            path: None,
            version: ver,
        });
    }

    // zstd level stored for debugging; not required for decode.
    let _zstd_level = i32::from_le_bytes(bytes[12..16].try_into().unwrap());

    let compressed = &bytes[16..];
    let decompressed =
        zstd::stream::decode_all(compressed).map_err(|e| ConfigError::Decompress(e.to_string()))?;

    let cfg: AppConfig =
        bitcode::decode(&decompressed).map_err(|e| ConfigError::Decode(e.to_string()))?;

    Ok(cfg)
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
            .unwrap_or("config")
    );
    tmp.set_file_name(tmp_name);

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    // On Windows, rename over existing can fail; remove first.
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(tmp, path)?;
    Ok(())
}
