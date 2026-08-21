//! Parity oracles for `hermes-time` vs upstream `tests/test_timezone.py`
//! (@ b9aa928). Mirrors the upstream test classes:
//!   TestHermesTimeNow   — env timezone → now() offset
//!   TestGetTimezone     — valid zone resolves; name available
//! plus config-file resolution and fallback behaviors.

use hermes_time::{get_timezone, get_timezone_name, now, reset_cache};
use std::sync::Mutex;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn clear_env() {
    unsafe {
        std::env::remove_var("HERMES_TIMEZONE");
        std::env::remove_var("HERMES_HOME");
    }
    reset_cache();
}

#[test]
fn valid_timezone_applies_ist() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("HERMES_TIMEZONE", "Asia/Kolkata") };
    let result = now();
    // IST is UTC+5:30
    assert_eq!(result.offset().local_minus_utc(), 5 * 3600 + 30 * 60);
    clear_env();
}

#[test]
fn utc_timezone() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("HERMES_TIMEZONE", "UTC") };
    assert_eq!(now().offset().local_minus_utc(), 0);
    clear_env();
}

#[test]
fn us_eastern_dst_aware() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("HERMES_TIMEZONE", "America/New_York") };
    let offset_hours = now().offset().local_minus_utc() as f64 / 3600.0;
    assert!(offset_hours == -5.0 || offset_hours == -4.0, "offset was {}", offset_hours);
    clear_env();
}

#[test]
fn get_timezone_returns_zone_for_valid() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("HERMES_TIMEZONE", "Europe/London") };
    assert!(get_timezone().is_some());
    assert_eq!(get_timezone_name().as_deref(), Some("Europe/London"));
    clear_env();
}

#[test]
fn invalid_timezone_falls_back_safely() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    unsafe { std::env::set_var("HERMES_TIMEZONE", "Not/AZone") };
    // No crash, no zone.
    assert!(get_timezone().is_none());
    // now() still returns a zone-aware value (server local).
    let result = now();
    let _offset = result.offset();
    clear_env();
}

#[test]
fn config_yaml_timezone_resolves() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    std::fs::write(td.path().join("config.yaml"), "timezone: Asia/Kolkata\n").unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };
    reset_cache();
    assert_eq!(get_timezone_name().as_deref(), Some("Asia/Kolkata"));
    assert_eq!(now().offset().local_minus_utc(), 5 * 3600 + 30 * 60);
    clear_env();
}

#[test]
fn unparseable_config_yaml_falls_back() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    std::fs::write(td.path().join("config.yaml"), ":: not yaml {").unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };
    reset_cache();
    assert!(get_timezone().is_none());
    clear_env();
}

#[test]
fn backward_compat_naive_unset() {
    let _g = ENV_MUTEX.lock().unwrap();
    clear_env();
    // Nothing configured: tz-aware local time, no zone.
    assert!(get_timezone().is_none());
    assert!(now().offset().local_minus_utc() != i32::MIN);
    clear_env();
}
