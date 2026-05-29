//! Legacy binary dataset format, kept only for one-time migration into the TOML workspace,
//! plus cross-platform state-directory paths.

pub mod dataset;
pub mod paths;

pub use dataset::{
    Collection, Dataset, DatasetFile, DatasetId, PersistError, PersistResult, Request,
};
pub use paths::{app_state_dir, default_dataset_path};
