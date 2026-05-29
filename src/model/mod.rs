//! Canonical, serde-(de)serializable domain model for a sasin workspace.
//!
//! Pure data, no I/O and no `reqwest`/`iced` types. The [`crate::storage`] layer maps these
//! types to and from the TOML directory tree; the [`crate::runtime`] layer resolves and executes
//! them. See `docs/planning/02-storage-format.md` for the on-disk schema.
//!
//! NOTE: `allow(dead_code)` is temporary — the model is consumed by the GUI in phase P2. Remove
//! the allow once `gui` reads/writes the workspace.
#![allow(dead_code)]

pub mod auth;
pub mod body;
pub mod defaults;
pub mod environment;
pub mod kv;
pub mod request;
pub mod scripts;
pub mod settings;
pub mod tree;
pub mod websocket;
pub mod workspace;

pub use auth::{ApiKeyLoc, Auth};
pub use body::{Body, FormKind, FormPart, RawLang};
pub use environment::{Environment, Variable};
pub use kv::KvEntry;
pub use request::HttpRequest;
pub use scripts::Scripts;
pub use settings::{Settings, WsSettings};
pub use tree::{Folder, Node, NodePath, find_node, find_node_mut, remove_node, resolve_auth};
pub use websocket::{WsKind, WsMessageTemplate, WsRequest};
pub use workspace::{Workspace, WorkspaceDefaults, WorkspaceManifest};
