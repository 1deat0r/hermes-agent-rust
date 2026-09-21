//! Request-level helpers shared by the auth routes and both middlewares.
//!
//! PARITY: `hermes_cli/dashboard_auth/request_utils.py` @ 5d59366 (whole
//! module, 89 lines). Transport-neutral by design: `client_ip` /
//! `extract_bearer` take header values (`Request` is the FastAPI surface),
//! `unreachable_response` is a status+body shape (the HTTP layer renders
//! it), `scan_session_providers` runs a caller-supplied sync closure
//! across the registry (async callers run it in a threadpool upstream).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::base::{DashboardAuthProvider, ProviderError};
use super::registry::list_session_providers;

/// Paths a post-login redirect must never land on: the auth flow itself
/// (would loop) and any `/api/*` target (raw JSON in the address bar,
/// indistinguishable from a weaponised redirect).
///
/// PARITY: `_NEXT_DENY_PREFIXES` (upstream lines 14-16).
pub const NEXT_DENY_PREFIXES: &[&str] = &["/login", "/auth/", "/api/auth/"];

/// First `X-Forwarded-For` hop, else the peer address.
///
/// PARITY: `client_ip` (upstream lines 19-22).
pub fn client_ip(x_forwarded_for: &str, peer_host: &str) -> String {
    if x_forwarded_for.is_empty() {
        return peer_host.to_string();
    }
    x_forwarded_for
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// `Authorization: Bearer <token>` value (scheme case-insensitive), or
/// `""`.
///
/// PARITY: `extract_bearer` (upstream lines 25-30).
pub fn extract_bearer(authorization_header: &str) -> String {
    let mut parts = authorization_header.splitn(2, ' ');
    match (parts.next(), parts.next()) {
        (Some(scheme), Some(token)) if scheme.trim().eq_ignore_ascii_case("bearer") => {
            token.trim().to_string()
        }
        _ => String::new(),
    }
}

/// Same-origin post-login target: rejects non-relative and
/// protocol-relative (`//evil`) values, the auth routes themselves,
/// and every `/api` path.
///
/// PARITY: `is_safe_next_path` (upstream lines 33-40).
pub fn is_safe_next_path(path: &str) -> bool {
    if !path.starts_with('/') || path.starts_with("//") {
        return false;
    }
    if NEXT_DENY_PREFIXES
        .iter()
        .any(|p| path == *p || path.starts_with(p))
    {
        return false;
    }
    !(path == "/api" || path.starts_with("/api/"))
}

/// Cookie Max-Age for the access token: seconds to `exp`, floored at 60.
///
/// PARITY: `access_token_max_age` (upstream lines 43-45).
pub fn access_token_max_age(expires_at: i64, now_unix: i64) -> i64 {
    60.max(expires_at.saturating_sub(now_unix))
}

/// Current unix time (the `time.time()` call site, explicit for tests).
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 503 body for a transient IDP/backing-store outage (never a forced
/// re-login). The HTTP layer renders this as JSON with status 503.
///
/// PARITY: `unreachable_response` (upstream lines 48-50).
pub fn unreachable_detail(provider_name: &str) -> String {
    format!("Auth provider {provider_name:?} unreachable")
}

/// HTTP status for [`unreachable_detail`].
pub const UNREACHABLE_STATUS: u16 = 503;

/// Outcome of scanning one provider.
pub enum ScanOutcome<T> {
    /// A definitive answer — stop scanning and return it.
    Done(T),
    /// This candidate rejects the credential — try the next provider.
    Next,
    /// This candidate is unreachable — remember and continue.
    Unreachable,
}

/// Run a closure across the session providers; first definitive answer
/// or `None`.
///
/// The hinted provider goes first (stable sort; a stale/unknown hint
/// leaves registration order intact). An unreachable provider must NOT
/// abort the chain — the credential may belong to a different,
/// reachable provider; if nothing else succeeds, `Err(ProviderError)`
/// so the caller answers 503 instead of forcing a re-login.
///
/// PARITY: `scan_session_providers` (upstream lines 53-89). The
/// `swallow`/`on_swallow`/`on_unreachable` hooks fold into
/// [`ScanOutcome`]: swallowed exceptions are `Next`, unreachable is
/// `Unreachable`.
pub fn scan_session_providers<T>(
    provider_hint: Option<&str>,
    mut call: impl FnMut(&Arc<dyn DashboardAuthProvider>) -> ScanOutcome<T>,
) -> Result<Option<T>, ProviderError> {
    let mut providers = list_session_providers();
    if let Some(hint) = provider_hint {
        if !hint.is_empty() {
            providers.sort_by_key(|p| p.name() != hint);
        }
    }
    let mut unreachable: Option<String> = None;
    for provider in &providers {
        match call(provider) {
            ScanOutcome::Done(value) => return Ok(Some(value)),
            ScanOutcome::Next => continue,
            ScanOutcome::Unreachable => {
                log::warn!(
                    "dashboard-auth: provider {:?} unreachable during scan",
                    provider.name()
                );
                if unreachable.is_none() {
                    unreachable = Some(provider.name().to_string());
                }
            }
        }
    }
    match unreachable {
        Some(name) => Err(ProviderError(name)),
        None => Ok(None),
    }
}
