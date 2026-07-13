//! Response-body capture: bounded read + text/binary classification.

use crate::models::ResponseBody;

/// Hard cap on retained response bytes (10 MiB). Larger payloads are truncated so a stray
/// multi-gigabyte download cannot take the app down; the flag is surfaced in the UI.
pub const MAX_CAPTURE_BYTES: usize = 10 * 1024 * 1024;

/// Whether a content type is inherently textual (shown as text even when not valid UTF-8).
pub fn is_texty_content_type(content_type: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    ct.starts_with("text/")
        || ct.contains("json")
        || ct.contains("xml")
        || ct.contains("html")
        || ct.contains("javascript")
        || ct.contains("x-www-form-urlencoded")
}

/// Classify captured bytes: text when the content type is texty (lossy if needed) or the
/// bytes are valid UTF-8; binary otherwise.
pub fn classify(content_type: Option<&str>, bytes: Vec<u8>) -> ResponseBody {
    let texty = content_type.is_some_and(is_texty_content_type);
    match String::from_utf8(bytes) {
        Ok(s) => ResponseBody::Text(s),
        Err(e) if texty => {
            // Texty but not UTF-8 (e.g. latin-1 HTML): keep it displayable.
            ResponseBody::Text(String::from_utf8_lossy(e.as_bytes()).into_owned())
        }
        Err(e) => ResponseBody::Binary(e.into_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_is_text_regardless_of_content_type() {
        let body = classify(Some("application/octet-stream"), b"plain text".to_vec());
        assert_eq!(body.as_text(), Some("plain text"));
        let body = classify(None, b"{\"a\":1}".to_vec());
        assert_eq!(body.as_text(), Some("{\"a\":1}"));
    }

    #[test]
    fn texty_content_type_is_lossy_decoded() {
        // 0xE9 = latin-1 'é', invalid on its own in UTF-8.
        let body = classify(
            Some("text/html; charset=latin1"),
            vec![b'c', b'a', b'f', 0xE9],
        );
        match body {
            ResponseBody::Text(s) => assert!(s.starts_with("caf")),
            ResponseBody::Binary(_) => panic!("texty content must classify as text"),
        }
    }

    #[test]
    fn invalid_utf8_without_texty_type_is_binary() {
        let png_magic = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let body = classify(Some("image/png"), png_magic.clone());
        assert_eq!(body, ResponseBody::Binary(png_magic));
    }

    #[test]
    fn texty_matrix() {
        for ct in [
            "text/plain",
            "application/json",
            "application/problem+json",
            "application/xml",
            "text/html; charset=utf-8",
            "application/javascript",
        ] {
            assert!(is_texty_content_type(ct), "{ct} should be texty");
        }
        for ct in ["image/png", "application/octet-stream", "application/pdf"] {
            assert!(!is_texty_content_type(ct), "{ct} should not be texty");
        }
    }
}
