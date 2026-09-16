//! Parity tests for `normalize_budget_warning_ratio`
//! (`agent/iteration_budget.py` @ 5d59366, lines 14-22).
//!
//! Oracle: source-as-oracle + live Python spot values (session log
//! 2026-09-16): None/bool → None; numeric strings coerce; finite and
//! strictly inside (0,1), else None. The `Any` input crosses the seam as
//! [`RatioInput`]; Rust `None` covers upstream `None`.

use hermes_agent::iteration_budget::{normalize_budget_warning_ratio, RatioInput};

/// None/bool disable the feature (None), even truthy bools.
#[test]
fn none_and_bools_disable() {
    assert_eq!(normalize_budget_warning_ratio(None), None);
    assert_eq!(
        normalize_budget_warning_ratio(Some(RatioInput::Bool(true))),
        None
    );
    assert_eq!(
        normalize_budget_warning_ratio(Some(RatioInput::Bool(false))),
        None
    );
}

/// Finite ratios strictly inside (0,1) pass through, incl. numeric strings.
#[test]
fn interior_ratios_pass_through() {
    assert_eq!(
        normalize_budget_warning_ratio(Some(RatioInput::Number(0.8))),
        Some(0.8)
    );
    assert_eq!(
        normalize_budget_warning_ratio(Some(RatioInput::Text("0.25"))),
        Some(0.25)
    );
}

/// Boundaries, non-finite, and junk → None.
#[test]
fn boundaries_and_junk_return_none() {
    for input in [
        RatioInput::Number(0.0),
        RatioInput::Number(1.0),
        RatioInput::Number(-0.5),
        RatioInput::Number(2.0),
        RatioInput::Number(f64::NAN),
        RatioInput::Number(f64::INFINITY),
        RatioInput::Text("abc"),
        RatioInput::Text(""),
    ] {
        assert_eq!(normalize_budget_warning_ratio(Some(input)), None);
    }
}
