//! Execute a full [`HttpRequest`] from the workspace model: build a client from the request's
//! [`Settings`](crate::model::Settings), apply auth/headers/params/body, send, and capture the response.

use std::path::Path;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};

use crate::http::auth::apply_auth;
use crate::http::body::apply_body;
use crate::http::client::HttpClientConfig;
use crate::model::{Auth, HttpRequest, KvEntry};
use crate::models::ResponseModel;

/// Build and send `request`, applying the already-resolved `auth`. File bodies resolve against
/// `base_dir` (the workspace directory). Returns a thin response model.
pub async fn execute(
    config: &HttpClientConfig,
    request: &HttpRequest,
    auth: &Auth,
    base_dir: &Path,
) -> Result<ResponseModel, String> {
    let client = build_client(config, request)?;
    let method = parse_method(&request.method)?;
    let url = effective_url(&request.url, &request.params)?;

    let mut rb = client.request(method, url);
    rb = apply_headers(rb, &request.headers)?;
    rb = apply_auth(rb, auth);
    rb = apply_body(rb, &request.body, base_dir)?;

    let start = Instant::now();
    let res = rb.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.text().await.map_err(|e| e.to_string())?;
    Ok(ResponseModel::new(
        status.as_u16(),
        reason(status),
        headers,
        body,
        start.elapsed(),
    ))
}

fn build_client(
    config: &HttpClientConfig,
    request: &HttpRequest,
) -> Result<reqwest::Client, String> {
    let s = &request.settings;
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_millis(s.timeout_ms.max(1)))
        .redirect(if s.follow_redirects {
            reqwest::redirect::Policy::default()
        } else {
            reqwest::redirect::Policy::none()
        })
        .danger_accept_invalid_certs(!s.verify_tls)
        .cookie_store(s.use_cookie_jar);

    if let Some(ua) = &config.user_agent {
        builder = builder.user_agent(ua.clone());
    }
    if let Some(proxy) = &s.proxy
        && !proxy.trim().is_empty()
    {
        let p = reqwest::Proxy::all(proxy.trim())
            .map_err(|e| format!("Invalid proxy `{proxy}`: {e}"))?;
        builder = builder.proxy(p);
    }

    builder.build().map_err(|e| e.to_string())
}

/// Parse an HTTP method, allowing custom verbs.
pub(crate) fn parse_method(method: &str) -> Result<reqwest::Method, String> {
    reqwest::Method::from_bytes(method.trim().to_ascii_uppercase().as_bytes())
        .map_err(|e| format!("Invalid method `{method}`: {e}"))
}

/// Merge enabled query params into the URL. Falls back to raw append when the base does not parse
/// (e.g. still contains `{{vars}}` before interpolation lands in P4).
pub(crate) fn effective_url(base: &str, params: &[KvEntry]) -> Result<String, String> {
    let enabled: Vec<(&str, &str)> = params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| (p.key.trim(), p.value.as_str()))
        .collect();
    if enabled.is_empty() {
        return Ok(base.to_string());
    }
    match reqwest::Url::parse(base) {
        Ok(mut url) => {
            url.query_pairs_mut().extend_pairs(enabled);
            Ok(url.to_string())
        }
        Err(_) => {
            let sep = if base.contains('?') { "&" } else { "?" };
            let query = enabled
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            Ok(format!("{base}{sep}{query}"))
        }
    }
}

fn apply_headers(
    mut rb: reqwest::RequestBuilder,
    headers: &[KvEntry],
) -> Result<reqwest::RequestBuilder, String> {
    for h in headers {
        if !h.enabled {
            continue;
        }
        let name = h.key.trim();
        if name.is_empty() {
            continue;
        }
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| format!("Invalid header name `{name}`: {e}"))?;
        let header_value = HeaderValue::from_str(h.value.trim())
            .map_err(|e| format!("Invalid header value for `{name}`: {e}"))?;
        rb = rb.header(header_name, header_value);
    }
    Ok(rb)
}

fn reason(status: reqwest::StatusCode) -> String {
    status.canonical_reason().unwrap_or("Unknown").to_string()
}
