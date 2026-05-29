//! Data-file parsing for data-driven runs: CSV (header row → keys) or JSON (array of objects).
//!
//! Each row becomes a flat `key -> string` map applied as per-iteration variable overrides.
//! JSON scalar values are stringified (strings verbatim, numbers/bools via their literal form);
//! nested arrays/objects are JSON-encoded so a script can still parse them.

use std::collections::HashMap;
use std::path::Path;

/// One data-file row: variable name → value.
pub type DataRow = HashMap<String, String>;

/// Parse a data file into rows, choosing the format by extension (`.json` → JSON, else CSV).
pub fn parse_data_file(path: &Path) -> Result<Vec<DataRow>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let is_json = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if is_json {
        parse_json(&text)
    } else {
        parse_csv(&text)
    }
}

/// Parse a JSON array of objects into rows.
pub fn parse_json(text: &str) -> Result<Vec<DataRow>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON data file: {e}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| "JSON data file must be an array of objects".to_string())?;
    let mut rows = Vec::with_capacity(array.len());
    for item in array {
        let object = item
            .as_object()
            .ok_or_else(|| "every JSON data row must be an object".to_string())?;
        let mut row = DataRow::new();
        for (key, val) in object {
            row.insert(key.clone(), json_scalar(val));
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Parse a CSV file (first record is the header) into rows.
pub fn parse_csv(text: &str) -> Result<Vec<DataRow>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| format!("invalid CSV header: {e}"))?
        .clone();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| format!("invalid CSV row: {e}"))?;
        let mut row = DataRow::new();
        for (key, field) in headers.iter().zip(record.iter()) {
            row.insert(key.to_string(), field.to_string());
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Render a JSON value as the string a variable should hold.
fn json_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
