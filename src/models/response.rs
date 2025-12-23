use std::time::Duration;

use reqwest::header::HeaderMap;

/// Thin response model used by the GUI.
///
/// This intentionally stores the body as `String` for quick display. If you later
/// need binary support, switch to `bytes::Bytes` plus a best-effort preview.
#[derive(Debug, Clone)]
pub struct ResponseModel {
    pub status: ResponseStatus,
    pub headers: HeaderMap,
    pub body: String,
    pub duration: Duration,
}

/// Normalized status info (useful for UI display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseStatus {
    pub code: u16,
    pub reason: String,
}

impl ResponseStatus {
    pub fn new(code: u16, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }

    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.code)
    }
}

impl ResponseModel {
    pub fn new(
        code: u16,
        reason: impl Into<String>,
        headers: HeaderMap,
        body: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            status: ResponseStatus::new(code, reason),
            headers,
            body: body.into(),
            duration,
        }
    }
}
