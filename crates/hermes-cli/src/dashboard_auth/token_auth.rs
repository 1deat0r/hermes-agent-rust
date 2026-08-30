//! Route-agnostic non-interactive (bearer-token) auth seam for the
//! dashboard.
//!
//! PARITY: `hermes_cli/dashboard_auth/token_auth.py` @ b9aa928 — PARTIAL.
//! Ported: the token-route registry, bearer-token extraction, and the
//! stacked provider authentication returning
//! `(principal, unreachable_provider)`. PENDING:
//! `token_auth_middleware`'s Request/JSONResponse plumbing — that is the
//! FastAPI web-server surface; the decision table it implements (valid
//! token → pass with principal, unreachable → 503, otherwise 401; the
//! cookie gates honour `token_authenticated`) rides the same values this
//! module returns.
//!
//! The generic API-token capability: ANY service-to-service /
//! machine-credential provider plugs into this seam — a route opts in by
//! registering its exact path, so this can never accidentally widen the
//! auth surface of an existing route. Fails closed: a token route with no
//! registered provider, no token, or an unrecognised token is a 401 —
//! never an open pass-through.

use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use super::audit::{audit_log, AuditEvent};
use super::base::{ProviderError, TokenPrincipal};
use super::registry::list_token_providers;

/// Exact paths that accept non-interactive bearer-token auth. Registering
/// a route does NOT make it public — it authenticates by token instead of
/// by session cookie.
///
/// PARITY: `_token_routes` / `_lock` (upstream lines 43-46).
static TOKEN_ROUTES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Mark `path` (exact match) as token-authable. Idempotent.
///
/// PARITY: `register_token_route` (upstream lines 49-58).
pub fn register_token_route(path: &str) {
    TOKEN_ROUTES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(path.to_string());
}

/// True if `path` was registered as token-authable (exact match).
///
/// PARITY: `is_token_route` (upstream lines 61-64).
pub fn is_token_route(path: &str) -> bool {
    TOKEN_ROUTES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(path)
}

/// Test-only: drop all registered token routes.
///
/// PARITY: `clear_token_routes` (upstream lines 67-70).
pub fn clear_token_routes() {
    TOKEN_ROUTES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// Return the bearer token from the `Authorization` header, or "".
///
/// Accepts `<scheme> <token>` where scheme is "bearer"
/// (case-insensitive). Returns empty for a missing/malformed header or a
/// non-bearer scheme — the caller treats "" as "no token presented".
///
/// PARITY: `extract_bearer_token` (upstream lines 82-93).
pub fn extract_bearer_token(authorization_header: Option<&str>) -> String {
    let Some(auth) = authorization_header else {
        return String::new();
    };
    let mut parts = auth.splitn(2, ' ');
    match (parts.next(), parts.next()) {
        (Some(scheme), Some(token)) if scheme.trim().eq_ignore_ascii_case("bearer") => {
            token.trim().to_string()
        }
        _ => String::new(),
    }
}

/// The provider-stacking outcome (upstream's `(principal, unreachable)`
/// tuple):
///   * `(Some(principal), None)` — a provider accepted the token.
///   * `(None, None)` — no token, or no provider recognised it (401).
///   * `(None, Some(name))` — no provider accepted it AND at least one
///     provider's backing store was unreachable (503, not 401, so a
///     transient outage doesn't read as "bad credentials").
///
/// Never panics: a provider `ProviderError` is remembered; a panicking
/// provider is fail-isolated like any other buggy provider.
///
/// PARITY: `authenticate_token` (upstream lines 96-138).
pub fn authenticate_token(
    token: &str,
    client_ip: &str,
    path: &str,
) -> (Option<TokenPrincipal>, Option<String>) {
    if token.is_empty() {
        return (None, None);
    }
    let mut unreachable: Option<String> = None;
    for provider in list_token_providers() {
        match futures::executor::block_on(provider.verify_token(token)) {
            Ok(Some(principal)) => return (Some(principal), None),
            // Not recognised — the seam moves on to the next provider.
            Ok(None) => continue,
            Err(err) => {
                log::warn!(
                    "dashboard-auth: token provider {:?} unreachable during verify: {}",
                    provider.name(),
                    err
                );
                if unreachable.is_none() {
                    unreachable = Some(provider.name().to_string());
                }
                continue;
            }
        }
    }
    let _ = (client_ip, path); // consumed by the middleware layer's audit call
    (None, unreachable)
}

/// Emit the audit record for a rejected token-auth attempt (the middleware
/// layer calls this where upstream's `token_auth_middleware` logs inline).
///
/// PARITY: the `TOKEN_AUTH_FAILURE` audit_log calls inside
/// `token_auth_middleware` (upstream lines 158-181) — provider=unreachable
/// + `provider_unreachable` for the 503 path, `no_provider_recognises_token`
/// for the 401 path.
pub fn audit_token_failure(unreachable: Option<&str>, path: &str, client_ip: &str) {
    match unreachable {
        Some(provider) => audit_log(
            AuditEvent::TokenAuthFailure,
            &[
                ("provider", serde_json::json!(provider)),
                ("reason", serde_json::json!("provider_unreachable")),
                ("path", serde_json::json!(path)),
                ("ip", serde_json::json!(client_ip)),
            ],
        ),
        None => audit_log(
            AuditEvent::TokenAuthFailure,
            &[
                ("reason", serde_json::json!("no_provider_recognises_token")),
                ("path", serde_json::json!(path)),
                ("ip", serde_json::json!(client_ip)),
            ],
        ),
    }
}

/// PARITY: `_client_ip` (upstream lines 73-79) — first
/// X-Forwarded-For entry when present, else the direct client host.
pub fn client_ip(x_forwarded_for: Option<&str>, direct_host: Option<&str>) -> String {
    if let Some(fwd) = x_forwarded_for {
        if !fwd.is_empty() {
            return fwd.split(',').next().unwrap_or("").trim().to_string();
        }
    }
    direct_host.unwrap_or("").to_string()
}
