//! JSON parse helpers.
//!
//! PARITY: utils.py @ 5d59366 — `safe_json_loads` (495–500),
//! `read_json_or_empty` (328–337).

use serde::de::DeserializeOwned;

/// Parse JSON, returning `default` on any parse error.
///
/// Mirrors the `try: json.loads(x) except (JSONDecodeError, TypeError)`
/// pattern.
///
/// PARITY: `safe_json_loads` (495–500).
pub fn safe_json_loads(text: &str, default: serde_json::Value) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or(default)
}

/// Generic typed variant of [`safe_json_loads`] for `T: DeserializeOwned`.
pub fn safe_json_loads_typed<T: DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_str(text).ok()
}

/// The JSON object at `path`, or `{}` when the file is missing,
/// unreadable, malformed, or not an object. The read half of every
/// read → merge → atomic_json_write config store, so a corrupt sidecar
/// degrades to defaults instead of taking the provider down.
///
/// A Windows-editor BOM must not wipe the config: the leading
/// `U+FEFF` (utf-8-sig) is stripped before parsing.
///
/// PARITY: `read_json_or_empty` (328–337).
pub fn read_json_or_empty(path: impl AsRef<Path>) -> serde_json::Value {
    let Ok(text) = std::fs::read_to_string(path.as_ref()) else {
        return serde_json::Value::Object(serde_json::Map::new());
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()))
}

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid() {
        assert_eq!(
            safe_json_loads(r#"{"a": 1}"#, serde_json::Value::Null)["a"],
            1
        );
    }

    #[test]
    fn returns_default_on_bad() {
        assert_eq!(
            safe_json_loads("not json", serde_json::Value::Null),
            serde_json::Value::Null
        );
        assert_eq!(
            safe_json_loads("", serde_json::json!({})),
            serde_json::json!({})
        );
    }

    #[test]
    fn read_json_or_empty_shapes() {
        let td = tempfile::TempDir::new().unwrap();
        let p = td.path().join("x.json");
        assert!(read_json_or_empty(&p).is_object());
        std::fs::write(&p, "[1]").unwrap();
        assert!(read_json_or_empty(&p).is_object());
        std::fs::write(&p, "{broken").unwrap();
        assert!(read_json_or_empty(&p).is_object());
        std::fs::write(&p, "\u{feff}{\"ok\":1}").unwrap();
        assert_eq!(read_json_or_empty(&p)["ok"].as_i64(), Some(1));
    }
}
