//! Tests for request construction (no network — built requests are inspected, not sent).

use std::path::Path;

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName};

use crate::http::auth::apply_auth;
use crate::http::body::apply_body;
use crate::http::exec::{effective_url, parse_method};
use crate::model::{ApiKeyLoc, Auth, Body, KvEntry, RawLang};

fn header(req: &reqwest::Request, name: HeaderName) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

#[test]
fn url_merges_enabled_params() -> Result<(), String> {
    assert_eq!(effective_url("https://h/p", &[])?, "https://h/p");

    let params = vec![
        KvEntry::new("a", "1"),
        KvEntry {
            key: "skip".into(),
            value: "x".into(),
            enabled: false,
        },
        KvEntry::new("b", "two"),
    ];
    let url = effective_url("https://h/p", &params)?;
    assert!(url.starts_with("https://h/p?"), "{url}");
    assert!(url.contains("a=1") && url.contains("b=two"));
    assert!(!url.contains("skip"));

    let merged = effective_url("https://h/p?z=0", &[KvEntry::new("a", "1")])?;
    assert!(merged.contains("z=0") && merged.contains("a=1"), "{merged}");
    Ok(())
}

#[test]
fn methods_parse_including_custom() -> Result<(), String> {
    assert_eq!(parse_method("get")?, Method::GET);
    assert_eq!(parse_method("PURGE")?.as_str(), "PURGE");
    Ok(())
}

fn built_with_auth(auth: &Auth) -> Result<reqwest::Request, String> {
    let client = reqwest::Client::new();
    let rb = client.request(Method::GET, "http://example.test/");
    apply_auth(rb, auth).build().map_err(|e| e.to_string())
}

#[test]
fn auth_sets_expected_headers() -> Result<(), String> {
    let bearer = built_with_auth(&Auth::Bearer {
        token: "t0ken".into(),
    })?;
    assert_eq!(
        header(&bearer, AUTHORIZATION).as_deref(),
        Some("Bearer t0ken")
    );

    let basic = built_with_auth(&Auth::Basic {
        user: "u".into(),
        pass: "p".into(),
    })?;
    assert!(
        header(&basic, AUTHORIZATION)
            .unwrap_or_default()
            .starts_with("Basic ")
    );

    assert_eq!(header(&built_with_auth(&Auth::None)?, AUTHORIZATION), None);

    let client = reqwest::Client::new();
    let rb = client.request(Method::GET, "http://example.test/");
    let api = apply_auth(
        rb,
        &Auth::ApiKey {
            key: "X-Key".into(),
            value: "secret".into(),
            add_to: ApiKeyLoc::Header,
        },
    )
    .build()
    .map_err(|e| e.to_string())?;
    assert_eq!(
        api.headers().get("X-Key").and_then(|v| v.to_str().ok()),
        Some("secret")
    );
    Ok(())
}

#[tokio::test]
async fn raw_body_sets_content_type_and_bytes() -> Result<(), String> {
    let client = reqwest::Client::new();
    let rb = client.request(Method::POST, "http://example.test/");
    let req = apply_body(
        rb,
        &Body::Raw {
            language: RawLang::Json,
            text: "{\"a\":1}".into(),
        },
        Path::new("/tmp"),
        false,
    )
    .await
    .and_then(|rb| rb.build().map_err(|e| e.to_string()))?;
    assert_eq!(
        header(&req, CONTENT_TYPE).as_deref(),
        Some("application/json")
    );
    assert_eq!(
        req.body().and_then(|b| b.as_bytes()).unwrap_or_default(),
        b"{\"a\":1}"
    );
    Ok(())
}

#[tokio::test]
async fn graphql_body_is_json_envelope() -> Result<(), String> {
    let client = reqwest::Client::new();
    let rb = client.request(Method::POST, "http://example.test/");
    let req = apply_body(
        rb,
        &Body::GraphQl {
            query: "{ me }".into(),
            variables: "{\"x\":1}".into(),
        },
        Path::new("/tmp"),
        false,
    )
    .await
    .and_then(|rb| rb.build().map_err(|e| e.to_string()))?;
    assert_eq!(
        header(&req, CONTENT_TYPE).as_deref(),
        Some("application/json")
    );
    let body = req.body().and_then(|b| b.as_bytes()).unwrap_or_default();
    let v: serde_json::Value = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    assert_eq!(v["query"], "{ me }");
    assert_eq!(v["variables"]["x"], 1);
    Ok(())
}
