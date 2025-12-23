use std::time::{Duration, Instant};

use reqwest::header::HeaderMap;

use crate::models::{HeaderEntry, HttpMethod, RequestModel, ResponseModel};

/// Small, explicit configuration surface.
/// Extend this as you add headers, proxy support, etc.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub timeout: Duration,
    pub user_agent: Option<String>,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            user_agent: Some("sasin/0.1".to_string()),
        }
    }
}

fn map_method(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    }
}

fn reason_or_fallback(status: reqwest::StatusCode) -> String {
    status.canonical_reason().unwrap_or("Unknown").to_string()
}

/// Send a request using a fresh `reqwest::Client` configured from `config`.
///
/// # Timing semantics
/// The returned `duration` measures **the full round-trip**, starting immediately
/// before `.send().await` and ending after the response body has been fully
/// downloaded via `.text().await`.
pub async fn send(
    config: &HttpClientConfig,
    request: RequestModel,
) -> Result<ResponseModel, String> {
    // Minimal pre-check: the GUI should already call `validate`, but keep this safe.
    request
        .validate()
        .map_err(|e| format!("Invalid request: {e}"))?;

    let mut builder = reqwest::Client::builder().timeout(config.timeout);

    if let Some(ua) = &config.user_agent {
        builder = builder.user_agent(ua.clone());
    }

    let client = builder.build().map_err(|e| e.to_string())?;

    let method = map_method(request.method);

    let mut rb = client.request(method, request.url);

    // Headers
    rb = apply_headers(rb, &request.headers)?;

    // Body (raw text)
    if let Some(body) = request.body {
        rb = rb.body(body);
    }

    let start = Instant::now();
    let res = rb.send().await.map_err(|e| e.to_string())?;

    let status = res.status();
    let headers: HeaderMap = res.headers().clone();
    let body = res.text().await.map_err(|e| e.to_string())?;

    // Full request duration including body download.
    let duration = start.elapsed();

    Ok(ResponseModel::new(
        status.as_u16(),
        reason_or_fallback(status),
        headers,
        body,
        duration,
    ))
}

fn apply_headers(
    mut rb: reqwest::RequestBuilder,
    headers: &[HeaderEntry],
) -> Result<reqwest::RequestBuilder, String> {
    for h in headers {
        let name = h.name.trim();
        let value = h.value.trim();

        if name.is_empty() {
            continue;
        }

        let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| format!("Invalid header name `{name}`: {e}"))?;

        let header_value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|e| format!("Invalid header value for `{name}`: {e}"))?;

        rb = rb.header(header_name, header_value);
    }

    Ok(rb)
}
