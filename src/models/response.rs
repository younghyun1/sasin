//! Thin response model used by the GUI, scripting, and the runner.

use std::borrow::Cow;
use std::time::Duration;

use reqwest::header::HeaderMap;

/// Captured response payload. Text is kept as `String` for direct display; anything that is
/// not valid UTF-8 (and not a texty content type) is retained as raw bytes so images and
/// other binaries can be previewed or saved to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseBody {
    Text(String),
    Binary(Vec<u8>),
}

impl ResponseBody {
    /// Byte length of the payload.
    pub fn len(&self) -> usize {
        match self {
            ResponseBody::Text(s) => s.len(),
            ResponseBody::Binary(b) => b.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The payload as text, when it is text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ResponseBody::Text(s) => Some(s),
            ResponseBody::Binary(_) => None,
        }
    }

    /// The raw payload bytes (text is its UTF-8 encoding).
    pub fn bytes(&self) -> &[u8] {
        match self {
            ResponseBody::Text(s) => s.as_bytes(),
            ResponseBody::Binary(b) => b,
        }
    }

    /// Text view for consumers that require a string (scripts, dumps): lossy for binary.
    pub fn text_lossy(&self) -> Cow<'_, str> {
        match self {
            ResponseBody::Text(s) => Cow::Borrowed(s),
            ResponseBody::Binary(b) => String::from_utf8_lossy(b),
        }
    }
}

impl From<&str> for ResponseBody {
    fn from(s: &str) -> Self {
        ResponseBody::Text(s.to_string())
    }
}

impl From<String> for ResponseBody {
    fn from(s: String) -> Self {
        ResponseBody::Text(s)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(b: Vec<u8>) -> Self {
        ResponseBody::Binary(b)
    }
}

/// Thin response model used by the GUI.
#[derive(Debug, Clone)]
pub struct ResponseModel {
    pub status: ResponseStatus,
    pub headers: HeaderMap,
    pub body: ResponseBody,
    /// The capture cap was hit; `body` holds only the first chunk of the payload.
    pub truncated: bool,
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
        body: impl Into<ResponseBody>,
        duration: Duration,
    ) -> Self {
        Self {
            status: ResponseStatus::new(code, reason),
            headers,
            body: body.into(),
            truncated: false,
            duration,
        }
    }
}
