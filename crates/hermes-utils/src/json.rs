//! JSON parse helpers.
//!
//! PARITY: utils.py lines 483–497 (`safe_json_loads`).

use serde::de::DeserializeOwned;

/// Parse JSON, returning `default` on any parse error.
///
/// Mirrors the `try: json.loads(x) except (JSONDecodeError, TypeError)` pattern.
///
/// PARITY: utils.py `safe_json_loads` (486–496).
pub fn safe_json_loads(text: &str, default: serde_json::Value) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or(default)
}

/// Generic typed variant of [`safe_json_loads`] for `T: DeserializeOwned`.
pub fn safe_json_loads_typed<T: DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_str(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid() {
        assert_eq!(safe_json_loads(r#"{"a": 1}"#, serde_json::Value::Null)["a"], 1);
    }

    #[test]
    fn returns_default_on_bad() {
        assert_eq!(safe_json_loads("not json", serde_json::Value::Null), serde_json::Value::Null);
        assert_eq!(safe_json_loads("", serde_json::json!({})), serde_json::json!({}));
    }
}
