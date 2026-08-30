//! Parity tests for `gateway/platforms/_http_client_limits.py` @ b9aa928,
//! mirroring the applicable cases in upstream
//! `tests/gateway/test_platform_http_client_limits.py` (the
//! `httpx unavailable → None` case has no Rust analog; the WhatsApp
//! `send_typing` source-inspection class belongs to that plugin module).
//! Env tests are serialized behind a mutex per the workspace convention.

use std::sync::Mutex;

use hermes_gateway::platform_http_limits::{
    platform_http_limits, DEFAULT_KEEPALIVE_EXPIRY_S, DEFAULT_MAX_KEEPALIVE,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// The upstream autouse `_clear_env` fixture: both override vars are
/// cleared before each test.
fn with_cleared_env<F: FnOnce()>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY");
        std::env::remove_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE");
    }
    f();
    unsafe {
        std::env::remove_var("HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY");
        std::env::remove_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE");
    }
}

#[test]
fn defaults_are_the_documented_tuned_values() {
    with_cleared_env(|| {
        let limits = platform_http_limits();
        assert_eq!(limits.keepalive_expiry, DEFAULT_KEEPALIVE_EXPIRY_S);
        assert_eq!(limits.max_keepalive_connections, DEFAULT_MAX_KEEPALIVE);
        assert_eq!(limits.keepalive_expiry, 2.0);
        assert_eq!(limits.max_keepalive_connections, 10);
    });
}

#[test]
fn env_override_rejects_garbage() {
    // Malformed env values fall back to defaults rather than raising.
    with_cleared_env(|| {
        unsafe {
            std::env::set_var("HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY", "not-a-number");
            std::env::set_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE", "-3");
        }
        let limits = platform_http_limits();
        assert!(limits.keepalive_expiry > 0.0);
        assert_eq!(limits.keepalive_expiry, 2.0);
        assert!(limits.max_keepalive_connections > 0);
        assert_eq!(limits.max_keepalive_connections, 10);
    });
}

#[test]
fn env_override_accepts_positive_values() {
    with_cleared_env(|| {
        unsafe {
            std::env::set_var("HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY", "4.5");
            std::env::set_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE", "32");
        }
        let limits = platform_http_limits();
        assert_eq!(limits.keepalive_expiry, 4.5);
        assert_eq!(limits.max_keepalive_connections, 32);
    });
}

#[test]
fn zero_and_blank_overrides_fall_back() {
    with_cleared_env(|| {
        unsafe {
            std::env::set_var("HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY", "0");
            std::env::set_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE", "   ");
        }
        let limits = platform_http_limits();
        assert_eq!(limits.keepalive_expiry, 2.0, "val if val > 0 else default");
        assert_eq!(limits.max_keepalive_connections, 10);
    });
}

#[test]
fn float_keepalive_parsed_as_int_falls_back() {
    // `_env_int("32.5")` — Python int("32.5") raises ValueError → default.
    with_cleared_env(|| {
        unsafe { std::env::set_var("HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE", "32.5") };
        assert_eq!(platform_http_limits().max_keepalive_connections, 10);
    });
}
