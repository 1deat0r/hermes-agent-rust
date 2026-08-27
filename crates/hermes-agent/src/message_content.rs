//! Visible-text extraction from chat/Responses message content shapes.
//!
//! PARITY: `agent/message_content.py` @ b9aa928 (whole module).

use serde_json::Value;

/// PARITY: `_NON_TEXT_PART_TYPES` (upstream line 6).
const NON_TEXT_PART_TYPES: [&str; 5] =
    ["image", "image_url", "input_image", "audio", "input_audio"];

/// PARITY: `_TEXT_KEYS` (upstream line 7), in lookup order.
const TEXT_KEYS: [&str; 5] = [
    "text",
    "content",
    "input_text",
    "output_text",
    "summary_text",
];

/// PARITY: `_field` (upstream lines 10-13). Python falls back to attribute
/// access on SDK part objects; the Rust wire representation is a mapping for
/// both shapes, so a non-object value simply has no fields.
fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key)
}

/// PARITY: `str(_field(part, "type") or "")` (upstream line 20). Falsy values
/// collapse to the empty string; a non-string type renders through JSON rather
/// than Python `str()` (no upstream shape depends on the difference).
fn part_type(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(false)) => String::new(),
        Some(Value::Number(number)) => {
            if number.as_f64() == Some(0.0) {
                String::new()
            } else {
                number.to_string()
            }
        }
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(true)) => "true".to_string(),
        Some(Value::Array(items)) if items.is_empty() => String::new(),
        Some(Value::Object(items)) if items.is_empty() => String::new(),
        Some(other) => other.to_string(),
    }
}

/// PARITY: `_text_from_part` (upstream lines 16-27).
fn text_from_part(part: &Value) -> String {
    if part.is_null() {
        return String::new();
    }
    if let Value::String(text) = part {
        return text.clone();
    }
    let normalized = part_type(field(part, "type")).trim().to_lowercase();
    if NON_TEXT_PART_TYPES.contains(&normalized.as_str()) {
        return String::new();
    }
    for key in TEXT_KEYS {
        if let Some(Value::String(text)) = field(part, key) {
            return text.clone();
        }
    }
    String::new()
}

/// Return the visible text from common chat/Responses message content shapes.
///
/// PARITY: `flatten_message_text` (upstream lines 30-47). `content` is the raw
/// payload (`None` for the Python `None` case); list content joins its
/// non-empty parts with `sep`, a single part falls back to its string form
/// when no text key matched, matching the source's `str(content)` tail that
/// never raises for JSON-shaped values.
pub fn flatten_message_text(content: Option<&Value>, sep: &str) -> String {
    let Some(content) = content else {
        return String::new();
    };
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => {
            let chunks: Vec<String> = parts
                .iter()
                .map(text_from_part)
                .filter(|chunk| !chunk.is_empty())
                .collect();
            chunks.join(sep)
        }
        other => {
            let text = text_from_part(other);
            if !text.is_empty() {
                return text;
            }
            other.to_string()
        }
    }
}
