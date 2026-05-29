//! curl import/export tests.

use crate::interop::{from_curl, to_curl};
use crate::model::{Auth, Body};

#[test]
fn imports_post_with_json_body() -> Result<(), String> {
    let req =
        from_curl("curl -X POST https://h/api -H 'Content-Type: application/json' -d '{\"a\":1}'")?;
    assert_eq!(req.method, "POST");
    assert_eq!(req.url, "https://h/api");
    assert!(req.headers.iter().any(|h| h.key == "Content-Type"));
    assert!(matches!(req.body, Body::Raw { .. }));
    Ok(())
}

#[test]
fn infers_post_when_data_present() -> Result<(), String> {
    let req = from_curl("curl https://h -d x=1")?;
    assert_eq!(req.method, "POST");
    Ok(())
}

#[test]
fn imports_basic_auth_and_form() -> Result<(), String> {
    let req = from_curl("curl https://h -u user:pass -F file=@a.bin -F k=v")?;
    assert!(matches!(req.auth, Auth::Basic { .. }));
    assert!(matches!(req.body, Body::FormData { .. }));
    Ok(())
}

#[test]
fn get_with_data_becomes_query_params() -> Result<(), String> {
    let req = from_curl("curl -G https://h -d q=hello")?;
    assert_eq!(req.method, "GET");
    assert!(
        req.params
            .iter()
            .any(|p| p.key == "q" && p.value == "hello")
    );
    Ok(())
}

#[test]
fn no_url_is_an_error() {
    assert!(from_curl("curl -X GET").is_err());
}

#[test]
fn multiline_continuation_parses() -> Result<(), String> {
    let req = from_curl("curl https://h \\\n  -H 'Accept: application/json'")?;
    assert_eq!(req.url, "https://h");
    assert!(req.headers.iter().any(|h| h.key == "Accept"));
    Ok(())
}

#[test]
fn export_then_reimport_round_trips() -> Result<(), String> {
    let req = from_curl("curl -X PUT 'https://h/x' -H 'Accept: application/json' -d 'body'")?;
    let curl = to_curl(&req);
    assert!(curl.contains("-X PUT"), "{curl}");
    assert!(curl.contains("https://h/x"));
    assert!(curl.contains("-H 'Accept: application/json'"));
    assert!(curl.contains("-d 'body'"));

    let again = from_curl(&curl)?;
    assert_eq!(again.method, "PUT");
    assert_eq!(again.url, "https://h/x");
    assert!(matches!(again.body, Body::Raw { .. }));
    Ok(())
}
