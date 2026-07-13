//! Pure text formatting for the response panel: byte counts, JSON pretty-printing,
//! line search, and the binary hex dump.

/// How many leading bytes of a binary body the hex preview shows.
pub(super) const HEX_PREVIEW_BYTES: usize = 1024;

/// Human-readable byte count for the stats chips.
pub(super) fn format_bytes(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

/// Best-effort body formatting: pretty-print JSON when enabled and parseable, else the raw body.
pub(super) fn format_body(body: &str, pretty_json: bool) -> String {
    if body.is_empty() {
        return "<empty body>".to_string();
    }
    if pretty_json
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(body)
        && let Ok(pretty) = serde_json::to_string_pretty(&value)
    {
        return pretty;
    }
    body.to_string()
}

/// Filter `text` to lines containing `query` (case-insensitive). Returns the text to show plus an
/// optional match-count header. An empty query returns the text unchanged.
pub(super) fn filter_search(text: &str, query: &str) -> (String, Option<String>) {
    let query = query.trim();
    if query.is_empty() {
        return (text.to_string(), None);
    }
    let needle = query.to_ascii_lowercase();
    let matches: Vec<&str> = text
        .lines()
        .filter(|line| line.to_ascii_lowercase().contains(&needle))
        .collect();
    let header = format!("{} matching line(s) for \"{query}\"", matches.len());
    (matches.join("\n"), Some(header))
}

/// Classic 16-bytes-per-line hex dump (offset, hex bytes, ASCII gutter) of the first `max` bytes.
pub(super) fn hex_dump(bytes: &[u8], max: usize) -> String {
    let slice = &bytes[..bytes.len().min(max)];
    let mut out = String::with_capacity(slice.len() * 4);
    for (i, line) in slice.chunks(16).enumerate() {
        let mut hex = String::with_capacity(48);
        let mut ascii = String::with_capacity(16);
        for b in line {
            hex.push_str(&format!("{b:02x} "));
            ascii.push(if b.is_ascii_graphic() || *b == b' ' {
                *b as char
            } else {
                '.'
            });
        }
        out.push_str(&format!("{:08x}  {hex:<48}  {ascii}\n", i * 16));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_render_human_scale() {
        assert_eq!(format_bytes(12), "12 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }

    #[test]
    fn hex_dump_caps_and_formats() {
        let dump = hex_dump(&[0x00, 0x41, 0xff], 16);
        assert!(dump.starts_with("00000000  00 41 ff"));
        assert!(dump.trim_end().ends_with(".A."));
        let capped = hex_dump(&[0u8; 64], 16);
        assert_eq!(capped.lines().count(), 1);
    }

    #[test]
    fn search_counts_matching_lines() {
        let (shown, header) = filter_search("alpha\nbeta\nALPHA beta", "alpha");
        assert_eq!(shown.lines().count(), 2);
        match header {
            Some(h) => assert!(h.starts_with("2 matching")),
            None => panic!("expected a match-count header"),
        }
    }
}
