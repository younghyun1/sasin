/// HTTP layer: thin wrapper around `reqwest`.
///
/// Kept deliberately small and testable. The GUI should call into this module
/// instead of using `reqwest` directly.
pub mod client;

pub use client::{HttpClientConfig, send};
