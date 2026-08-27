// Tier: unit — mirrors tests/agent/test_reasoning_summaries.py.

use hermes_agent::reasoning_summaries::separate_glued_reasoning_blocks;

/// Accumulate deltas the way the chat-completions stream loop does (the
/// upstream `_stream` helper).
fn stream(deltas: &[&str]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for delta in deltas {
        let tail = parts.last().map(String::as_str).unwrap_or("");
        parts.push(separate_glued_reasoning_blocks(tail, delta));
    }
    parts.concat()
}

#[test]
fn heading_only_parts_do_not_glue_into_one_run() {
    let text = stream(&[
        "**Investigating likely culprit PRs**",
        "**Inspecting message schema and tool_calls content**",
        "**Analyzing interrupted tool call impact**",
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
        "**Simulating a greeting stream**\n\nIt feels like a streaming interaction!",
        "**Simulating a greeting stream**\n\nI want to meet the request.",
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
    assert_eq!(stream(&deltas), deltas.concat());
}

#[test]
fn bold_word_mid_sentence_is_not_a_boundary() {
    assert_eq!(
        separate_glued_reasoning_blocks("I see the ", "**signature**"),
        "**signature**"
    );
}

#[test]
fn unclosed_emphasis_fragment_is_not_a_boundary() {
    assert_eq!(separate_glued_reasoning_blocks("weighing", "**"), "**");
}

#[test]
fn boundary_needs_a_bold_opener() {
    assert_eq!(
        separate_glued_reasoning_blocks("**Closing**", "plain head"),
        "plain head"
    );
}

#[test]
fn empty_operands_pass_through() {
    assert_eq!(
        separate_glued_reasoning_blocks("", "**first**"),
        "**first**"
    );
    assert_eq!(separate_glued_reasoning_blocks("**first**", ""), "");
}
