//! HTTP layer: build and execute requests from the workspace model.
//!
//! The GUI calls [`execute`] with a resolved [`HttpRequest`](crate::model::HttpRequest); this
//! module owns all `reqwest` usage (client construction, auth, body encoding).

pub mod auth;
pub mod body;
pub mod client;
pub mod exec;

#[cfg(test)]
mod tests;

pub use client::HttpClientConfig;
pub use exec::execute;
