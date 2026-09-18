// Tier: unit — mirrors tests/agent/test_reasoning_summaries.py.

use hermes_agent::reasoning_summaries::separate_glued_reasoning_blocks;

/// Accumulate deltas the way the chat-completions stream loop does (the
/// upstream `_stream` helper).
fn stream(deltas: &[serde_json::Value]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for delta in deltas {
        let tail = parts.last().map(String::as_str).unwrap_or("");
        parts.push(separate_glued_reasoning_blocks(tail, delta));
    }
    parts.concat()
}
fn s(text: &str) -> serde_json::Value {
    serde_json::Value::String(text.to_string())
}

#[test]
fn heading_only_parts_do_not_glue_into_one_run() {
    let text = stream(&[
        s("**Investigating likely culprit PRs**"),
        s("**Inspecting message schema and tool_calls content**"),
        s("**Analyzing interrupted tool call impact**"),
    ]);
    assert!(!text.contains("****"));
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        vec![
            "**Investigating likely culprit PRs**",
            "",
            "**Inspecting message schema and tool_calls content**",
            "",
            "**Analyzing interrupted tool call impact**",
        ]
    );
}

#[test]
fn prose_body_does_not_glue_onto_the_next_heading() {
    let text = stream(&[
        s("**Simulating a greeting stream**\n\nIt feels like a streaming interaction!"),
        s("**Simulating a greeting stream**\n\nI want to meet the request."),
    ]);
    assert!(!text.contains("interaction!**"));
    assert!(text.contains("interaction!\n\n**Simulating"));
}

#[test]
fn token_streamed_reasoning_is_untouched() {
    let deltas = [
        "Looking at",
        " the session",
        " logs, I see",
        " one bold word.",
    ];
    let values: Vec<serde_json::Value> = deltas.iter().map(|d| s(d)).collect();
    assert_eq!(stream(&values), deltas.concat());
}

#[test]
fn bold_word_mid_sentence_is_not_a_boundary() {
    assert_eq!(
        separate_glued_reasoning_blocks("I see the ", &s("**signature**")),
        "**signature**"
    );
}

#[test]
fn unclosed_emphasis_fragment_is_not_a_boundary() {
    assert_eq!(separate_glued_reasoning_blocks("weighing", &s("**")), "**");
}

#[test]
fn boundary_needs_a_bold_opener() {
    assert_eq!(
        separate_glued_reasoning_blocks("**Closing**", &s("plain head")),
        "plain head"
    );
}

#[test]
fn empty_operands_pass_through() {
    assert_eq!(
        separate_glued_reasoning_blocks("", &s("**first**")),
        "**first**"
    );
    assert_eq!(separate_glued_reasoning_blocks("**first**", &s("")), "");
}

/// PARITY @ 5d59366: `delta` is `Any` — relays emit content-part lists and
/// dicts; `flatten_message_text(delta, sep="")` normalizes first.
#[test]
fn any_shaped_deltas_flatten_before_boundary_check() {
    use serde_json::json;
    assert_eq!(
        separate_glued_reasoning_blocks("prev", &json!([{"type": "text", "text": "**Head**"}])),
        "\n\n**Head**"
    );
    assert_eq!(
        separate_glued_reasoning_blocks("prev", &json!({"type": "text", "text": "**Head** tail"})),
        "\n\n**Head** tail"
    );
    assert_eq!(
        separate_glued_reasoning_blocks("", &json!([{"type": "text", "text": "x"}])),
        "x"
    );
    // Seam edge (no Python shape: Python None short-circuits to ""): a JSON
    // null value falls to the `str(content)` tail → "null".
    assert_eq!(
        separate_glued_reasoning_blocks("prev", &json!(null)),
        "null"
    );
    assert_eq!(separate_glued_reasoning_blocks("prev", &json!(42)), "42");
}

/// PARITY @ 5d59366: `append_streamed_reasoning_detail` — consecutive
/// same-type fragments merge (later backfills signature/id); encrypted or
/// foreign entries stay discrete; non-dicts drop.
#[test]
fn streamed_detail_fragments_merge_with_backfill() {
    use hermes_agent::reasoning_summaries::append_streamed_reasoning_detail;
    use serde_json::json;
    let mut acc = Vec::new();
    append_streamed_reasoning_detail(
        &mut acc,
        json!({"type": "reasoning.text", "text": "hello", "signature": "s1"}),
    );
    append_streamed_reasoning_detail(
        &mut acc,
        json!({"type": "reasoning.text", "text": " world", "id": "i1"}),
    );
    append_streamed_reasoning_detail(&mut acc, json!({"type": "other", "x": 1}));
    assert_eq!(acc.len(), 2);
    assert_eq!(acc[0]["text"], "hello world");
    assert_eq!(acc[0]["signature"], "s1");
    assert_eq!(acc[0]["id"], "i1");

    let mut acc2 = Vec::new();
    append_streamed_reasoning_detail(&mut acc2, json!("not-a-dict"));
    assert!(acc2.is_empty());
}
