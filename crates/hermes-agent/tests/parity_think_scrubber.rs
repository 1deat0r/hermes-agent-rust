//! Parity tests for `agent/think_scrubber.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle, including the
//! docstring's MiniMax-M2.7 split-delta failure scenario.

use hermes_agent::think_scrubber::StreamingThinkScrubber;

/// Feed deltas and flush; return (per-delta emissions, flush output).
fn run(deltas: &[&str]) -> (Vec<String>, String) {
    let mut scrubber = StreamingThinkScrubber::new();
    let emissions: Vec<String> = deltas
        .iter()
        .map(|d| {
            let visible = scrubber.feed(d);
            (!visible.is_empty()).then_some(visible).unwrap_or_default()
        })
        .collect();
    let tail = scrubber.flush();
    (emissions, tail)
}

#[test]
fn split_deltas_never_leak_reasoning() {
    // The docstring scenario: the opener arrives alone in delta1 — a
    // per-delta regex erases it and leaks delta2 to the user.
    let (emissions, tail) = run(&[
        "<think>",
        "Let me check their config",
        "</think>",
        "Answer: 42",
    ]);
    assert!(emissions.iter().all(|e| !e.contains("config")));
    assert_eq!(emissions.concat() + &tail, "Answer: 42");
}

#[test]
fn complete_block_in_one_delta_is_suppressed() {
    let (emissions, tail) = run(&["<think>internal reasoning</think>Visible reply."]);
    assert_eq!(emissions.concat() + &tail, "Visible reply.");
}

#[test]
fn closed_pair_inline_in_prose_is_suppressed_without_boundaries() {
    // A closed pair is intentional and bounded — suppressed even mid-line.
    let (emissions, tail) = run(&["use <think>tags</think> here for reasoning, please continue"]);
    assert_eq!(
        emissions.concat() + &tail,
        "use  here for reasoning, please continue"
    );
}

#[test]
fn prose_mentioning_the_tag_at_mid_line_is_not_stripped() {
    // "<think>" mid-line (no boundary) is ordinary prose.
    let (emissions, tail) = run(&["you can use <think> tags here"]);
    assert_eq!(emissions.concat() + &tail, "you can use <think> tags here");
}

#[test]
fn open_after_newline_is_a_block_boundary() {
    let (emissions, tail) = run(&["Sure.\n<think>hidden</think>Final: 7\n"]);
    assert_eq!(emissions.concat() + &tail, "Sure.\nFinal: 7\n");
}

#[test]
fn unclosed_block_discards_held_content_on_flush() {
    let (emissions, tail) = run(&["<think>", "partial reasoning never closed"]);
    assert!(emissions.iter().all(|e| !e.contains("partial")));
    assert_eq!(tail, "", "unterminated block discards held-back content");
}

#[test]
fn held_back_partial_tag_resolves_across_deltas() {
    // A split tag: "<thi" held back, "nk>" resolves the block entry.
    let (emissions, tail) = run(&["Hello <thi", "nk>secret</think>", " world"]);
    assert!(emissions
        .iter()
        .all(|e| !e.contains("<thi") && !e.contains("secret")));
    assert_eq!(emissions.concat() + &tail, "Hello  world");
}

#[test]
fn held_back_non_tag_is_emitted_verbatim_on_flush() {
    // A trailing "<" is held back, then flushed verbatim when it turns out
    // not to be a tag prefix.
    let (emissions, tail) = run(&["5 < 10 and 12 > 3"]);
    assert_eq!(emissions.concat() + &tail, "5 < 10 and 12 > 3");
}

#[test]
fn flush_resets_the_boundary_flag_for_intra_turn_retries() {
    let mut scrubber = StreamingThinkScrubber::new();
    let _ = scrubber.feed("<think>x</think>text");
    let _ = scrubber.flush();
    // Retry: a fresh stream's opening <think> must not look mid-line.
    let visible = scrubber.feed("<think>new reasoning</think>retry answer");
    assert_eq!(visible, "retry answer");
}

#[test]
fn case_insensitive_variants_and_orphan_close_stripping() {
    // Case-insensitive variants.
    let (emissions, tail) = run(&["<THINK>x</THINK>ok"]);
    assert_eq!(emissions.concat() + &tail, "ok");
    // Orphan close tag (no matching open) is stripped with trailing space.
    let (emissions, tail) = run(&["before </think> after"]);
    assert_eq!(emissions.concat() + &tail, "before after");
}

#[test]
fn reasoning_scratchpad_variant_scrubs() {
    let mut scrubber = StreamingThinkScrubber::new();
    let visible = scrubber.feed("<REASONING_SCRATCHPAD>chain</REASONING_SCRATCHPAD>out");
    assert_eq!(visible, "out");
}

#[test]
fn reset_clears_state_between_turns() {
    let mut scrubber = StreamingThinkScrubber::new();
    let _ = scrubber.feed("<think>hung");
    scrubber.reset();
    // A hung block from an interrupted prior stream cannot taint the next.
    assert_eq!(scrubber.feed("fresh turn"), "fresh turn");
}

#[test]
fn whitespace_only_line_then_open_is_boundary() {
    // Only whitespace emitted on the current line: the open is at a
    // boundary.
    let (emissions, tail) = run(&["  \n<think>z</think>", "done"]);
    assert_eq!(emissions.concat() + &tail, "  \ndone");
}
