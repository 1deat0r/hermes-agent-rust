//! Parity tests for `hermes_cli/timefmt.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); these cases derive from the upstream code as oracle, using the
//! explicit-clock `_at` forms.

use hermes_cli::timefmt::relative_time_at;

const NOW: f64 = 1_800_000_000.0;

#[test]
fn falsy_timestamps_render_as_question_mark() {
    // Python `if not ts: return "?"` — None and 0.0.
    assert_eq!(relative_time_at(None, NOW), "?");
    assert_eq!(relative_time_at(Some(0.0), NOW), "?");
}

#[test]
fn sub_minute_is_just_now() {
    assert_eq!(relative_time_at(Some(NOW - 30.0), NOW), "just now");
    // A future timestamp has negative delta, which is < 60 -> "just now".
    assert_eq!(relative_time_at(Some(NOW + 120.0), NOW), "just now");
    assert_eq!(relative_time_at(Some(NOW - 59.9), NOW), "just now");
}

#[test]
fn minutes_and_hours_truncate() {
    // int(delta / 60) truncates: 119.9s -> 1m.
    assert_eq!(relative_time_at(Some(NOW - 60.0), NOW), "1m ago");
    assert_eq!(relative_time_at(Some(NOW - 119.9), NOW), "1m ago");
    assert_eq!(relative_time_at(Some(NOW - 3599.9), NOW), "59m ago");
    // int(delta / 3600).
    assert_eq!(relative_time_at(Some(NOW - 3600.0), NOW), "1h ago");
    assert_eq!(relative_time_at(Some(NOW - 86399.9), NOW), "23h ago");
}

#[test]
fn one_to_seven_days_use_yesterday_then_day_counts() {
    // 24h..<48h reads "yesterday" regardless of the exact hour count.
    assert_eq!(relative_time_at(Some(NOW - 86_400.0), NOW), "yesterday");
    assert_eq!(relative_time_at(Some(NOW - 172_799.9), NOW), "yesterday");
    // int(delta / 86400): 2..7 days.
    assert_eq!(relative_time_at(Some(NOW - 172_800.0), NOW), "2d ago");
    assert_eq!(relative_time_at(Some(NOW - 604_799.9), NOW), "6d ago");
}

#[test]
fn beyond_a_week_renders_the_local_date() {
    // datetime.fromtimestamp(...).strftime("%Y-%m-%d") in the local zone;
    // just pin the grammar, not the zone-dependent day.
    let old = relative_time_at(Some(NOW - 30.0 * 86_400.0), NOW);
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(re.is_match(&old), "{old}");
}

#[test]
fn fractional_and_integer_timestamps_behave_alike() {
    assert_eq!(
        relative_time_at(Some(1_800_000_123.45), 1_800_000_153.45),
        "just now"
    );
}
