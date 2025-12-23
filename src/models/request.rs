use std::fmt;

use bitcode::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

/// HTTP methods supported by the app.
///
/// Keep this small and ergonomic for the GUI (method picker) and the HTTP layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Encode, Decode)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    /// Stable list for rendering method pickers.
    pub const fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
            HttpMethod::Head,
            HttpMethod::Options,
        ]
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Minimal request model for a Postman-like "send request" flow.
///
/// Still lean, but supports basic headers and an optional body (raw text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestModel {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub body: Option<String>,
}

impl Default for RequestModel {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            url: String::new(),
            headers: Vec::new(),
            body: None,
        }
    }
}

impl RequestModel {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn with_headers(mut self, headers: Vec<HeaderEntry>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: Option<String>) -> Self {
        self.body = body;
        self
    }

    /// Basic validation to give quick feedback before sending.
    pub fn validate(&self) -> Result<(), &'static str> {
        let url = self.url.trim();
        if url.is_empty() {
            return Err("URL is empty");
        }
        // Keep validation lightweight; the HTTP layer will do full parsing/errors.
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("URL must start with http:// or https://");
        }

        for h in &self.headers {
            if h.name.trim().is_empty() {
                return Err("Header name cannot be empty");
            }
        }

        Ok(())
    }
}
