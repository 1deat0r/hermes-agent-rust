//! Parity tests for `agent/jiter_preload.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (no dedicated test file — gap noted). Contract:
//! best-effort early load of the native streaming parser; idempotent;
//! never raises (fail-open); reports success bool; keeps the last error
//! for diagnostics. The native `jiter` extension has no Rust counterpart —
//! the loader crosses the seam as an injected closure, and the default
//! loader performs the Rust-side equivalent warmup (JSON engine touch).

use hermes_agent::jiter_preload::{preload_jiter_native_extension as preload, preload_with, reset_for_tests, last_error};
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

/// First successful load reports true and sticks (idempotent).
#[test]
fn successful_preload_is_idempotent() {
    let _g = LOCK.lock().unwrap();
    reset_for_tests();
    let mut calls = 0;
    let result = preload_with(|| {
        calls += 1;
        Ok(())
    });
    assert!(result);
    assert_eq!(calls, 1);
    // Second call short-circuits without re-invoking the loader.
    assert!(preload_with(|| {
        calls += 1;
        Ok(())
    }));
    assert_eq!(calls, 1);
}

/// Failure is fail-open: returns false, records the error, never panics.
#[test]
fn failed_preload_is_fail_open_and_records_error() {
    let _g = LOCK.lock().unwrap();
    reset_for_tests();
    let result: bool = preload_with(|| Err("native ext missing".to_string()));
    assert!(!result);
    assert_eq!(last_error(), Some("native ext missing".to_string()));
}

/// A later success clears the recorded error (upstream resets both on ok).
#[test]
fn success_clears_recorded_error() {
    let _g = LOCK.lock().unwrap();
    reset_for_tests();
    let _: bool = preload_with(|| Err("boom".to_string()));
    assert!(preload_with(|| Ok(())));
    assert_eq!(last_error(), None);
}

/// Default loader performs the Rust-side warmup and reports honestly.
#[test]
fn default_preload_warms_up_without_raising() {
    let _g = LOCK.lock().unwrap();
    reset_for_tests();
    let _ = preload();
}
