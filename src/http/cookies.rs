//! A shared, enumerable cookie jar.
//!
//! reqwest's own `Jar` persists cookies but cannot be iterated, so the cookie-manager UI would
//! have nothing to show. We instead implement reqwest's [`CookieStore`](reqwest::cookie::CookieStore)
//! trait directly over [`cookie_store::CookieStore`] (the same crate + version reqwest uses
//! internally) wrapped in an `Arc<RwLock<…>>`, which we can also read to list/clear cookies.

use std::sync::{Arc, RwLock};

use cookie_store::{CookieDomain, CookieStore};
use reqwest::Url;
use reqwest::cookie::CookieStore as ReqwestCookieStore;
use reqwest::header::HeaderValue;

/// A single stored cookie, flattened for display in the manager. `domain` is the store's
/// raw key (the bare host for host-only cookies), so it round-trips into [`SharedCookieJar::remove`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieView {
    pub domain: String,
    pub host_only: bool,
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
                domain: String::from(&c.domain),
                host_only: matches!(c.domain, CookieDomain::HostOnly(_)),
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

    /// Remove one cookie by its identity triple. Host-only cookies list as `(host-only)` in
    /// the view; their stored domain is the bare host, so both are tried.
    pub fn remove(&self, domain: &str, path: &str, name: &str) -> bool {
        self.write().remove(domain, path, name).is_some()
    }

    /// Insert a cookie by parsing a `Set-Cookie`-style string against a synthetic URL for
    /// `domain`/`path` (used by the manager's add row).
    pub fn add(&self, domain: &str, path: &str, name: &str, value: &str) -> Result<(), String> {
        let domain = domain.trim().trim_start_matches('.');
        if domain.is_empty() || name.trim().is_empty() {
            return Err("domain and name are required".to_string());
        }
        let path = if path.trim().is_empty() {
            "/"
        } else {
            path.trim()
        };
        let url = Url::parse(&format!("https://{domain}{path}"))
            .map_err(|e| format!("invalid domain/path: {e}"))?;
        let header = format!("{}={}; Domain={domain}; Path={path}", name.trim(), value);
        self.write()
            .parse(&header, &url)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Serialize the jar (including session cookies) as JSON for persistence.
    pub fn to_json(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&self.read(), &mut out)
            .map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// Replace the jar contents from persisted JSON.
    pub fn load_json(&self, bytes: &[u8]) -> Result<(), String> {
        let store = cookie_store::serde::json::load_all(bytes).map_err(|e| e.to_string())?;
        *self.write() = store;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_and_snapshot() {
        let jar = SharedCookieJar::new();
        if let Err(e) = jar.add("api.example.com", "/", "sid", "abc") {
            panic!("add failed: {e}");
        }
        let snap = jar.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].name, "sid");
        assert!(jar.remove(&snap[0].domain, &snap[0].path, &snap[0].name));
        assert_eq!(jar.count(), 0);
    }

    #[test]
    fn add_validates_inputs() {
        let jar = SharedCookieJar::new();
        assert!(jar.add("", "/", "n", "v").is_err());
        assert!(jar.add("example.com", "/", "", "v").is_err());
    }

    #[test]
    fn json_round_trip_preserves_cookies() {
        let jar = SharedCookieJar::new();
        if let Err(e) = jar.add("example.com", "/app", "token", "t1") {
            panic!("add failed: {e}");
        }
        let json = match jar.to_json() {
            Ok(j) => j,
            Err(e) => panic!("to_json failed: {e}"),
        };
        let restored = SharedCookieJar::new();
        if let Err(e) = restored.load_json(&json) {
            panic!("load_json failed: {e}");
        }
        assert_eq!(restored.snapshot(), jar.snapshot());
    }
}
