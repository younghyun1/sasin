//! A shared, enumerable cookie jar.
//!
//! reqwest's own `Jar` persists cookies but cannot be iterated, so the cookie-manager UI would
//! have nothing to show. We instead implement reqwest's [`CookieStore`](reqwest::cookie::CookieStore)
//! trait directly over [`cookie_store::CookieStore`] (the same crate + version reqwest uses
//! internally) wrapped in an `Arc<RwLock<…>>`, which we can also read to list/clear cookies.

use std::sync::{Arc, RwLock};

use cookie_store::CookieStore;
use reqwest::Url;
use reqwest::cookie::CookieStore as ReqwestCookieStore;
use reqwest::header::HeaderValue;

/// A single stored cookie, flattened for display in the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieView {
    pub domain: String,
    pub name: String,
    pub value: String,
    pub path: String,
}

/// Session-wide cookie jar shared by every request client built this session.
#[derive(Clone)]
pub struct SharedCookieJar(Arc<RwLock<CookieStore>>);

impl SharedCookieJar {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(CookieStore::default())))
    }

    /// Snapshot the stored cookies for the manager view.
    pub fn snapshot(&self) -> Vec<CookieView> {
        let store = self.read();
        store
            .iter_any()
            .map(|c| CookieView {
                domain: c.domain().unwrap_or("(host-only)").to_string(),
                name: c.name().to_string(),
                value: c.value().to_string(),
                path: c.path().unwrap_or("/").to_string(),
            })
            .collect()
    }

    /// Remove every stored cookie.
    pub fn clear(&self) {
        self.write().clear();
    }

    /// Number of stored cookies (expired or not).
    pub fn count(&self) -> usize {
        self.read().iter_any().count()
    }

    // Lock helpers that recover the guard on poisoning rather than panicking.
    fn read(&self) -> std::sync::RwLockReadGuard<'_, CookieStore> {
        self.0.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, CookieStore> {
        self.0.write().unwrap_or_else(|p| p.into_inner())
    }
}

impl Default for SharedCookieJar {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SharedCookieJar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedCookieJar({} cookies)", self.count())
    }
}

impl ReqwestCookieStore for SharedCookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let mut store = self.write();
        for header in cookie_headers {
            if let Ok(text) = header.to_str() {
                let _ = store.parse(text, url);
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.read();
        let joined = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if joined.is_empty() {
            return None;
        }
        HeaderValue::from_str(&joined).ok()
    }
}
