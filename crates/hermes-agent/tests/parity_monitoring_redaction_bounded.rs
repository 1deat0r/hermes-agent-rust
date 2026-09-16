//! Parity tests for `redact_bounded` + rewired `redact_for_export`
//! (`agent/monitoring/redaction.py` @ 5d59366).
//!
//! Oracle: live Python outputs at the pin (independent literals — session
//! log 2026-09-16). `redact_bounded` redacts `str(raw or "")`, substitutes
//! `empty` for empty results, truncates to `limit` (no suffix), and returns
//! `unavailable` if redaction raises (infallible in Rust — contract only).

use hermes_agent::monitoring::redaction::{redact_bounded, redact_for_export};

/// Live oracle: 600 x's truncate to exactly 500 (default limit, no suffix).
#[test]
fn bounded_default_truncates_to_500() {
    let out = redact_bounded(&"x".repeat(600), 500, "[redacted]", "[redaction-unavailable]");
    assert_eq!(out.len(), 500);
    assert!(out.chars().all(|c| c == 'x'));
}

/// Live oracle: empty input yields the `empty` substitute.
#[test]
fn bounded_empty_yields_substitute() {
    assert_eq!(
        redact_bounded("", 500, "[redacted]", "[redaction-unavailable]"),
        "[redacted]"
    );
}

/// Live oracle: custom limit/empty/unavailable shape.
#[test]
fn bounded_custom_shape() {
    assert_eq!(
        redact_bounded("hello world, this is long", 10, "E", "U"),
        "hello worl"
    );
}

/// Rewired export: secrets via `redact_for_egress` (force + bearer sweep).
/// Live oracle: long opaque bearer folds after force-masking.
#[test]
fn export_folds_long_bearer() {
    let out = redact_for_export(Some("Authorization: Bearer abcdefghijklmnopqrst1234")).unwrap();
    assert!(!out.contains("abcdefghijklmnopqrst1234"), "{out}");
    assert!(out.contains("Bearer"), "{out}");
}

/// Short bearer words stay untouched through the rewired path.
#[test]
fn export_keeps_short_bearer_words() {
    assert_eq!(
        redact_for_export(Some("short bearer word here")),
        Some("short bearer word here".to_string())
    );
}
