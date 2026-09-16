//! Boundary repair for providers that stream reasoning as discrete summary
//! parts.
//!
//! PARITY: `agent/reasoning_summaries.py` @ 5d59366 (whole module, incl.
//! `append_streamed_reasoning_detail`).
//!
//! Reasoning-summary models (the gpt-5.x family and anything relaying the
//! Responses API onto the OpenAI chat wire) emit one `reasoning_content` delta
//! per *completed* summary part, each opening with a bold markdown heading.
//! The Responses API delimits those parts with `summary_index`; the chat wire
//! carries no such field, so consumers that concatenate deltas glue the parts
//! into `**One****Two**`, which no markdown parser reads as a close plus an
//! open. The boundary is re-derived here from the one signal the chat wire
//! does carry: a delta that opens — and closes — a bold heading while the
//! accumulated text is still mid-line.

use serde_json::{Map, Value};

use crate::message_content::flatten_message_text;

/// Return `delta`, prefixed with a paragraph break when it glues onto
/// `previous`.
///
/// PARITY: `separate_glued_reasoning_blocks` (upstream lines 20-30).
/// `delta` is `Any` upstream — relays emit content-part lists/dicts and
/// fragments carry their own whitespace — so it is flattened first
/// (`flatten_message_text(delta, sep="")`); `previous` is accumulated text,
/// only its tail matters. Token-streamed reasoning is left alone, and an
/// emphasis fragment that never closes in one delta is not a part boundary.
pub fn separate_glued_reasoning_blocks(previous: &str, delta: &Value) -> String {
    // Relays emit content-part lists/dicts; fragments carry whitespace.
    let delta = flatten_message_text(Some(delta), "");
    if previous.is_empty() || delta.is_empty() {
        return delta;
    }
    if !delta.starts_with("**") {
        return delta;
    }
    // Already separated — the provider (or an earlier part) ended the line.
    // `str.isspace()` on the final character; Rust's `char::is_whitespace`
    // covers the same Unicode whitespace set the source sees here.
    if previous
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_whitespace())
    {
        return delta;
    }
    // Require a *closed* heading: `delta[2:]` after the opening marker must
    // still contain `**`.
    if !delta[2..].contains("**") {
        return delta;
    }
    format!("\n\n{delta}")
}

/// Detail entry types whose consecutive fragments are one logical block.
///
/// PARITY: `_MERGEABLE_DETAIL_TEXT_KEYS` (upstream line 35).
const MERGEABLE_DETAIL_TEXT_KEYS: [(&str, &str); 2] = [
    ("reasoning.text", "text"),
    ("reasoning.summary", "summary"),
];

/// Keys a later fragment backfills when the first omitted them.
///
/// PARITY: `_BACKFILL_DETAIL_KEYS` (upstream line 36).
const BACKFILL_DETAIL_KEYS: [&str; 4] = ["signature", "id", "format", "index"];

/// Accumulate one streamed `reasoning_details` delta entry.
///
/// PARITY: `append_streamed_reasoning_detail` (upstream lines 39-67).
/// Consecutive same-type `reasoning.text` / `reasoning.summary` fragments
/// merge (later fragments backfill `signature`/`id`/… the first omitted);
/// encrypted/opaque entries stay discrete. Non-dict details drop
/// (SDK objects normalized upstream via `model_dump`/`__dict__` have no
/// Rust analog — callers pass `Value`). Unmerged, a long thought replays
/// as hundreds of one-word entries and shape-validating providers reject
/// the next turn.
pub fn append_streamed_reasoning_detail(details_acc: &mut Vec<Map<String, Value>>, detail: Value) {
    let Value::Object(detail) = detail else {
        return;
    };
    let dtype = detail.get("type").and_then(Value::as_str).unwrap_or("");
    let merge_key = MERGEABLE_DETAIL_TEXT_KEYS
        .iter()
        .find(|(kind, _)| *kind == dtype)
        .map(|(_, key)| *key);
    if let (Some(key), Some(last)) = (merge_key, details_acc.last_mut()) {
        let same_type = last.get("type").and_then(Value::as_str).unwrap_or("") == dtype;
        if same_type && detail.get(key).and_then(Value::as_str).is_some() {
            let merged = format!(
                "{}{}",
                last.get(key).and_then(Value::as_str).unwrap_or(""),
                detail.get(key).and_then(Value::as_str).unwrap_or("")
            );
            last.insert(key.to_string(), Value::String(merged));
            for backfill in BACKFILL_DETAIL_KEYS {
                let last_empty = last
                    .get(backfill)
                    .is_none_or(|v| v.is_null() || v == "");
                let incoming = detail.get(backfill);
                if last_empty
                    && incoming.is_some_and(|v| !v.is_null() && v != "")
                {
                    last.insert(backfill.to_string(), incoming.cloned().unwrap());
                }
            }
            return;
        }
    }
    details_acc.push(detail);
}
