//! Parity tests for `redact_for_egress` + `REDACTION_UNAVAILABLE`
//! (`agent/redact.py` @ 5d59366, lines ~1088-1105).
//!
//! Oracle: live Python outputs captured at the pin (independent literals —
//! see session log 2026-09-16). The 20-char floor is the documented
//! rationale: the English word "bearer" must not trigger masking.

use hermes_logging::{redact_for_egress, REDACTION_UNAVAILABLE};

/// The unavailable sentinel is pinned literally (fail-closed marker).
#[test]
fn unavailable_sentinel_is_pinned() {
    assert_eq!(REDACTION_UNAVAILABLE, "[redaction-unavailable]");
}

/// Empty in, empty out (`str(text or "")` then sweeps).
#[test]
fn empty_stays_empty() {
    assert_eq!(redact_for_egress(""), "");
}

/// The 20-char floor: plain English "bearer" never masks.
#[test]
fn short_bearer_word_is_untouched() {
    assert_eq!(redact_for_egress("the bearer of bad news"), "the bearer of bad news");
    assert_eq!(redact_for_egress("Bearer short"), "Bearer short");
}

/// Opaque 20+ char bearer folds to one marker (live oracle output).
#[test]
fn long_opaque_bearer_folds_to_marker() {
    assert_eq!(
        redact_for_egress("Bearer abcdefghijklmnopqrst1234"),
        "Bearer [redacted]"
    );
}

/// Already-masked residue folds to one marker (live oracle output).
#[test]
fn bracket_residue_folds_to_marker() {
    assert_eq!(redact_for_egress("Bearer [redacted-jwt]"), "Bearer [redacted]");
}
