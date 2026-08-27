// Tier: unit — mirrors tests/agent/test_manual_compression_feedback.py.

use hermes_agent::manual_compression_feedback::{
    describe_compression_lock_skip, summarize_manual_compression, CompressionStateSignals,
};
use serde_json::{json, Value};

fn messages(count: usize) -> Vec<Value> {
    (0..count)
        .map(|index| {
            json!({
                "role": if index % 2 == 0 { "user" } else { "assistant" },
                "content": index.to_string(),
            })
        })
        .collect()
}

#[test]
fn failure_reason_redaction_is_forced_at_the_ui_boundary() {
    let before = messages(12);
    let fake_secret = format!("sk-proj-{}", "X".repeat(40));
    let state = CompressionStateSignals::present()
        .with_aborted(true)
        .with_summary_error(format!("provider rejected OPENAI_API_KEY={fake_secret}"));

    let feedback = summarize_manual_compression(&before, &before, 120_000, 120_000, Some(&state));

    let note = feedback.note.expect("aborted compression carries a note");
    assert!(!note.contains(&fake_secret), "secret leaked into {note}");
    assert!(note.contains("OPENAI_API_KEY="));
    assert!(note.contains("Reason:"));
    assert!(note.contains("Summary generation failed; no messages were removed."));
}

#[test]
fn fallback_compression_reports_the_dropped_message_count() {
    let before = messages(12);
    let mut after = before[..2].to_vec();
    after.extend_from_slice(&before[before.len() - 2..]);
    let state = CompressionStateSignals::present()
        .with_fallback_used(true)
        .with_dropped_count(8)
        .with_summary_error("summary provider returned an invalid response");

    let feedback = summarize_manual_compression(&before, &after, 120_000, 40_000, Some(&state));

    assert!(!feedback.aborted);
    assert!(feedback.fallback_used);
    assert!(!feedback.noop);
    assert_eq!(
        feedback.headline,
        "Compressed with fallback: 12 → 4 messages"
    );
    let note = feedback.note.as_deref().expect("note");
    assert!(note.contains("removed 8 message(s)"));
    assert!(note.contains("invalid response"));
}

#[test]
fn headline_and_token_line_shapes() {
    let before = messages(12);
    let after = before[..4].to_vec();

    let plain = summarize_manual_compression(&before, &after, 120_000, 40_000, None);
    assert_eq!(plain.headline, "Compressed: 12 → 4 messages");
    assert_eq!(
        plain.token_line,
        "Approx request size: ~120,000 → ~40,000 tokens"
    );
    assert_eq!(plain.note, None);

    let aborted = summarize_manual_compression(
        &before,
        &before,
        120_000,
        120_000,
        Some(&CompressionStateSignals::present().with_aborted(true)),
    );
    assert!(aborted.aborted);
    assert_eq!(
        aborted.headline,
        "Compression aborted: 12 messages preserved"
    );
    assert_eq!(
        aborted.token_line,
        "Approx request size: ~120,000 tokens (unchanged)"
    );
    assert_eq!(
        aborted.note.as_deref(),
        Some("Summary generation failed; no messages were removed.")
    );

    let noop = summarize_manual_compression(&before, &before, 900, 1_200, None);
    assert!(noop.noop);
    assert_eq!(noop.headline, "No changes from compression: 12 messages");
    // A noop with a changed estimate still uses the arrow form.
    assert_eq!(noop.token_line, "Approx request size: ~900 → ~1,200 tokens");
}

#[test]
fn denser_summary_note_only_for_real_shrinks() {
    let before = messages(12);
    let after = before[..4].to_vec();
    let feedback = summarize_manual_compression(&before, &after, 40_000, 60_000, None);
    assert_eq!(
        feedback.note.as_deref(),
        Some(
            "Note: fewer messages can still raise this estimate when compression \
             rewrites the transcript into denser summaries."
        )
    );
    // No shrink in message count means no note.
    let same = summarize_manual_compression(&before, &before, 40_000, 60_000, None);
    assert_eq!(same.note, None);
}

#[test]
fn dropped_count_falls_back_to_the_message_delta() {
    let before = messages(12);
    let after = before[..4].to_vec();
    // No dropped-count signal at all.
    let state = CompressionStateSignals::present().with_fallback_used(true);
    let feedback = summarize_manual_compression(&before, &after, 120_000, 40_000, Some(&state));
    assert!(feedback
        .note
        .as_deref()
        .expect("note")
        .contains("removed 8 message(s)"));

    // A non-integer signal is ignored; Python also rejects `True` (bool is an
    // int subclass upstream, which the guard excludes deliberately).
    let bool_count = CompressionStateSignals::present()
        .with_fallback_used(true)
        .with_dropped_count(json!(true));
    let feedback =
        summarize_manual_compression(&before, &after, 120_000, 40_000, Some(&bool_count));
    assert!(feedback
        .note
        .as_deref()
        .expect("note")
        .contains("removed 8 message(s)"));
}

#[test]
fn blank_failure_reason_is_no_reason() {
    let before = messages(4);
    let state = CompressionStateSignals::present()
        .with_aborted(true)
        .with_summary_error("   ");
    let feedback = summarize_manual_compression(&before, &before, 10, 10, Some(&state));
    assert_eq!(
        feedback.note.as_deref(),
        Some("Summary generation failed; no messages were removed.")
    );
    // An aborted reason is never appended without a failure signal either.
    let clean = summarize_manual_compression(
        &before,
        &before,
        10,
        10,
        Some(&CompressionStateSignals::present()),
    );
    assert_eq!(clean.note, None);
    assert_eq!(clean.headline, "No changes from compression: 4 messages");
}

#[test]
fn lock_skip_wording_distinguishes_confirmed_holders() {
    assert_eq!(
        describe_compression_lock_skip(Some("tui:CompressionLockHeld")),
        "⏳ Compression already in progress for this session (holder: \
         tui:CompressionLockHeld). Please wait for it to finish."
    );
    for unconfirmed in [None, Some(""), Some("   ")] {
        assert_eq!(
            describe_compression_lock_skip(unconfirmed),
            "⏳ Compression skipped: could not acquire this session's compression \
             lock. Another compression may still be running, or the lock check \
             failed — try again shortly."
        );
    }
}
