//! Parity tests for `coerce_epoch` (`hermes_cli/timefmt.py` @ 5d59366).
//!
//! Oracle: live Python outputs at the pin (independent literals — session
//! log 2026-09-16). `datetime` inputs have no Rust analog (caller converts
//! to epoch); the `None`/`""`-silent and warning arms are pinned here.

use hermes_cli::timefmt::{coerce_epoch, EpochInput};

/// None/"" stay silent (no warning observable — None return, no log probe
/// needed for the value contract).
#[test]
fn unset_values_stay_silent_none() {
    assert_eq!(coerce_epoch(EpochInput::Empty, None, "timestamp"), None);
}

/// 0 coerces to 0.0 (in range, no warning) — falsiness is relative_time's job.
#[test]
fn zero_is_valid_epoch() {
    assert_eq!(
        coerce_epoch(EpochInput::Number(0.0), Some("s1"), "timestamp"),
        Some(0.0)
    );
    assert_eq!(
        coerce_epoch(EpochInput::Text("0"), None, "timestamp"),
        Some(0.0)
    );
}

/// Numbers and numeric strings (incl. whitespace) coerce.
#[test]
fn numbers_and_numeric_strings_coerce() {
    assert_eq!(
        coerce_epoch(EpochInput::Number(1700000000.5), None, "timestamp"),
        Some(1700000000.5)
    );
    assert_eq!(
        coerce_epoch(EpochInput::Text("1700000000"), None, "timestamp"),
        Some(1700000000.0)
    );
    assert_eq!(
        coerce_epoch(EpochInput::Text(" 1700000000 "), None, "timestamp"),
        Some(1700000000.0)
    );
}

/// Out-of-range, non-finite, and junk → None (warning arm is log-side).
#[test]
fn corrupt_values_return_none() {
    for input in [
        EpochInput::Number(-5.0),
        EpochInput::Number(99999999999.0),
        EpochInput::Number(f64::NAN),
        EpochInput::Number(f64::INFINITY),
        EpochInput::Text("abc"),
    ] {
        assert_eq!(coerce_epoch(input, Some("s1"), "timestamp"), None);
    }
}

/// Range bounds: EPOCH_MIN=0.0 and EPOCH_MAX=4_200_000_000.0 inclusive.
#[test]
fn range_bounds_are_inclusive() {
    assert_eq!(
        coerce_epoch(EpochInput::Number(4_200_000_000.0), None, "timestamp"),
        Some(4_200_000_000.0)
    );
    assert_eq!(
        coerce_epoch(EpochInput::Number(4_200_000_001.0), None, "timestamp"),
        None
    );
}
