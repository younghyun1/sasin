//! App configuration persistence.
//!
//! User-specific settings that are not part of the main dataset:
//! - window size/position
//! - last opened dataset file
//! - autosave enabled
//!
//! On disk format:
//! - magic: b"SASINCF1" (8 bytes)
//! - u32: format version (LE)
//! - bytes: bitcode payload of `AppConfig`
//!
//! Notes:
//! - No compression on this file; it's tiny.
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bitcode::{Decode, Encode};

use crate::persist::dataset::Request;
use crate::persist::{Dataset, DatasetFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Encode, Decode)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Encode, Decode)]
pub struct LayoutState {
    pub sidebar_width_px: u32,
    pub request_height_px: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct AppConfig {
    pub version: u32,
    pub window: Option<WindowState>,
    pub layout: Option<LayoutState>,
    pub last_dataset_path: Option<String>,
    pub autosave_enabled: bool,
}

/// File wrapper and persistence API.
pub struct AppConfigFile {
    path: PathBuf,
}

impl AppConfigFile {
    pub const MAGIC: [u8; 8] = *b"SASINCF1";
    pub const FORMAT_VERSION: u32 = 1;

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_default(&self) -> ConfigResult<AppConfig> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }
        self.load()
    }

    pub fn load(&self) -> ConfigResult<AppConfig> {
        let bytes = fs::read(&self.path).map_err(|e| ConfigError::Io(self.path.clone(), e))?;
        decode_config_file(&bytes).map_err(|e| e.with_path(self.path.clone()))
    }

    pub fn save(&self, config: &AppConfig) -> ConfigResult<()> {
        let bytes = encode_config_file(config)?;
        fs::write(&self.path, &bytes).map_err(|e| ConfigError::Io(self.path.clone(), e))
    }
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
    Encode(String),
    Decode(String),
}

impl ConfigError {
    fn with_path(mut self, path: PathBuf) -> Self {
        match &mut self {
            ConfigError::Io(p, _) => *p = path,
            ConfigError::InvalidFormat { path: p, .. } => *p = Some(path),
            ConfigError::UnsupportedVersion { path: p, .. } => *p = Some(path),
            ConfigError::Encode(_) => {}
            ConfigError::Decode(_) => {}
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
            ConfigError::Encode(msg) => write!(f, "Encode error: {msg}"),
            ConfigError::Decode(msg) => write!(f, "Decode error: {msg}"),
        }
    }
}

fn encode_config_file(config: &AppConfig) -> ConfigResult<Vec<u8>> {
    let payload = bitcode::encode(config);
    let mut out = Vec::with_capacity(8 + 4 + payload.len());
    out.extend_from_slice(&AppConfigFile::MAGIC);
    out.extend_from_slice(&AppConfigFile::FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

fn decode_config_file(bytes: &[u8]) -> ConfigResult<AppConfig> {
    if bytes.len() < 12 {
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

    let payload = &bytes[12..];
    bitcode::decode(payload).map_err(|e| ConfigError::Decode(e.to_string()))
}

/// On startup, load the last-opened dataset (or a default one).
///
/// Returns (path, dataset) or a user-facing error message string.
pub fn load_startup_dataset(
    config: &AppConfig,
    default_path: PathBuf,
) -> Result<(PathBuf, Dataset), String> {
    let path = config
        .last_dataset_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or(default_path);

    let file = DatasetFile::new(&path);
    let mut ds = file.load_or_default().map_err(|e| e.to_string())?;

    // Ensure at least one request exists for a good first-time user experience.
    if ds.collections.is_empty() {
        let mut new_collection = crate::persist::Collection {
            id: ds.next_id(),
            name: "My Collection".to_string(),
            requests: Vec::new(),
        };
        new_collection.requests.push(Request::new(
            ds.next_id(),
            "Default",
            crate::models::HttpMethod::Get,
            "https://example.com",
        ));
        ds.collections.push(new_collection);
        file.save(&ds).map_err(|e| e.to_string())?;
    }

    Ok((path, ds))
}
