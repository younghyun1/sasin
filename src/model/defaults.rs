//! Shared serde `default` helpers for the workspace model.
//!
//! These back `#[serde(default = "…")]` so that hand-edited TOML files with omitted
//! keys deserialize to the same values the app would have written.

/// Default for boolean fields whose natural state is `true` (e.g. `enabled`).
pub(crate) const fn default_true() -> bool {
    true
}

/// Default HTTP request timeout in milliseconds.
pub(crate) const fn default_timeout_ms() -> u64 {
    30_000
}

/// Default WebSocket connect timeout in milliseconds.
pub(crate) const fn default_connect_timeout_ms() -> u64 {
    5_000
}

/// Default HTTP method for a request whose file omits `method`.
pub(crate) fn default_get() -> String {
    "GET".to_string()
}
