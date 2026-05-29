//! HTTP client configuration. Per-request transport options (timeout, redirects, TLS, proxy,
//! cookies) live on [`crate::model::Settings`] and are applied in [`crate::http::exec`].

use crate::http::cookies::SharedCookieJar;

/// Workspace-wide client defaults that are not part of an individual request.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// User-Agent sent with every request (unless overridden by a request header).
    pub user_agent: Option<String>,
    /// Session-wide cookie jar, shared across requests when their cookie jar is enabled.
    pub jar: SharedCookieJar,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            user_agent: Some(concat!("sasin/", env!("CARGO_PKG_VERSION")).to_string()),
            jar: SharedCookieJar::new(),
        }
    }
}
