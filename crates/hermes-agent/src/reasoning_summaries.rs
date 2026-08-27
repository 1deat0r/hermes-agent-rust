//! Boundary repair for providers that stream reasoning as discrete summary
//! parts.
//!
//! PARITY: `agent/reasoning_summaries.py` @ b9aa928 (whole module, lines 1-68).
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

/// Return `delta`, prefixed with a paragraph break when it glues onto
/// `previous`.
///
/// PARITY: `separate_glued_reasoning_blocks` (upstream lines 33-68). `previous`
/// is the reasoning text accumulated so far; only its tail matters.
/// Token-streamed reasoning is left alone: its deltas carry their own leading
/// whitespace, and an emphasis fragment that never closes in one delta is not a
/// part boundary.
pub fn separate_glued_reasoning_blocks(previous: &str, delta: &str) -> String {
    if previous.is_empty() || delta.is_empty() {
        return delta.to_string();
    }
    if !delta.starts_with("**") {
        return delta.to_string();
    }
    // Already separated — the provider (or an earlier part) ended the line.
    // `str.isspace()` on the final character; Rust's `char::is_whitespace`
    // covers the same Unicode whitespace set the source sees here.
    if previous
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_whitespace())
    {
        return delta.to_string();
    }
    // Require a *closed* heading: `delta[2:]` after the opening marker must
    // still contain `**`.
    if !delta[2..].contains("**") {
        return delta.to_string();
    }
    format!("\n\n{delta}")
}
