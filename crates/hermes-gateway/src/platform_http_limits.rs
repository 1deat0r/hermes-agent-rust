//! Shared HTTP client pool limits for long-lived platform adapters.
//!
//! PARITY: `gateway/platforms/_http_client_limits.py` @ b9aa928 (whole
//! module).
//!
//! Gateway messaging platforms (QQ Bot, Feishu, WeCom, DingTalk, Signal,
//! BlueBubbles, WeCom-callback) keep a persistent HTTP client alive for the
//! adapter's lifetime. That amortises TLS/connection setup across many API
//! calls, but it also means the process's file-descriptor pressure is
//! sensitive to how aggressively the pool recycles idle keep-alive
//! connections (#18451).
//!
//! TRANSLATION NOTE: upstream returns an `httpx.Limits`; Rust has no single
//! HTTP-stack equivalent at this layer, so the port returns a plain
//! [`HttpPoolLimits`] struct carrying the same two knobs with the same
//! defaults and env-var grammar. The `httpx is None → return None`
//! ImportError arm has no Rust analog (the struct is dependency-free), so
//! callers always receive limits. Values chosen (see upstream):
//! `max_keepalive_connections = 10` — plenty for any single adapter;
//! `keepalive_expiry = 2.0` — close idle sockets aggressively so a proxy's
//! lingering CLOSE_WAIT window can't starve the process.
//!
//! Override via `HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY` /
//! `HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE` env vars when tuning under load.

/// PARITY: `_DEFAULT_KEEPALIVE_EXPIRY_S` (upstream line 32).
pub const DEFAULT_KEEPALIVE_EXPIRY_S: f64 = 2.0;

/// PARITY: `_DEFAULT_MAX_KEEPALIVE` (upstream line 33).
pub const DEFAULT_MAX_KEEPALIVE: i64 = 10;

/// Pool limits tuned for persistent platform-adapter clients.
///
/// PARITY: the `httpx.Limits(...)` fields set upstream
/// (`max_keepalive_connections`, `keepalive_expiry`); `max_connections`
/// stays at the HTTP stack's default (100), as the upstream comment notes.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpPoolLimits {
    pub max_keepalive_connections: i64,
    pub keepalive_expiry: f64,
}

/// PARITY: the `_env_float` closure (upstream lines 50-57). Blank, non-
/// numeric, and non-positive values all fall back to the default.
fn env_float(name: &str, default: f64) -> f64 {
    let raw = std::env::var(name).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return default;
    }
    match raw.parse::<f64>() {
        Ok(val) if val > 0.0 => val,
        _ => default,
    }
}

/// PARITY: the `_env_int` closure (upstream lines 59-66). Blank, non-
/// numeric, and non-positive values all fall back to the default.
fn env_int(name: &str, default: i64) -> i64 {
    let raw = std::env::var(name).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return default;
    }
    match raw.parse::<i64>() {
        Ok(val) if val > 0 => val,
        _ => default,
    }
}

/// Return pool limits tuned for persistent platform-adapter clients.
///
/// PARITY: `platform_httpx_limits` (upstream lines 69-85).
pub fn platform_http_limits() -> HttpPoolLimits {
    HttpPoolLimits {
        keepalive_expiry: env_float(
            "HERMES_GATEWAY_HTTPX_KEEPALIVE_EXPIRY",
            DEFAULT_KEEPALIVE_EXPIRY_S,
        ),
        max_keepalive_connections: env_int(
            "HERMES_GATEWAY_HTTPX_MAX_KEEPALIVE",
            DEFAULT_MAX_KEEPALIVE,
        ),
    }
}
