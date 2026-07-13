//! Legacy binary dataset format, kept only for one-time migration into the TOML workspace,
//! plus cross-platform state-directory paths and the TOML app preferences file.

pub mod config;
pub mod dataset;
pub mod paths;

pub use config::{ThemeChoice, UiPrefs, load_prefs, save_prefs};
pub use dataset::{
    Collection, Dataset, DatasetFile, DatasetId, PersistError, PersistResult, Request,
};
pub use paths::{app_state_dir, default_dataset_path};
