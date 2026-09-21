//! Auth-gate middleware decision tables for the dashboard.
//!
//! PARITY: `hermes_cli/dashboard_auth/middleware.py` @ 5d59366 (whole
//! module, 251 lines). Transport-neutral by design: the FastAPI
//! `Request`/`Response` plumbing (middleware runner, cookie I/O, thread
//! pool) belongs to the web-server surface; every *decision* the gate
//! makes ports here as pure functions over explicit inputs, and the
//! gate flow itself ports as [`GateDecision`] — the web surface
//! executes it and performs the I/O.
//!
//! The PLUGIN-COMPAT block (upstream lines 232-251) is intentionally
//! unported: compat pointers are off limits in-tree
//! (`scripts/check_compat_pointers.py`).

use super::public_paths::is_public_api_path;
use super::request_utils::is_safe_next_path;

/// Prefix-matched bypass list: auth bootstrap routes and static asset
/// mounts. `/assets/` with the trailing slash matches
/// `/assets/foo.css` but not `/assetsleak`.
///
/// PARITY: `_GATE_PUBLIC_PREFIXES` (upstream lines 37-43).
pub const GATE_PUBLIC_PREFIXES: &[&str] = &[
    "/auth/login",
    "/auth/callback",
    "/auth/native/authorize",
    "/auth/native/token",
    "/auth/native/refresh",
    "/auth/password-login",
    "/auth/logout",
    "/login",
    "/api/auth/providers",
    "/api/mcp/oauth/callback/",
    "/assets/",
    "/favicon.ico",
    "/ds-assets/",
    "/fonts/",
    "/fonts-terminal/",
];

/// `PUBLIC_API_PATHS` (shared with the legacy middleware) matched
/// exactly so `/api/status` never exposes `/api/status/extension`;
/// `_GATE_PUBLIC_PREFIXES` prefix-matched.
///
/// PARITY: `_path_is_public` (upstream lines 46-51).
pub fn path_is_public(path: &str) -> bool {
    is_public_api_path(path)
        || GATE_PUBLIC_PREFIXES
            .iter()
            .any(|p| path == *p || path.starts_with(p))
}

/// Percent-encode a `next` target the way the gate does (`quote(...,
/// safe="")` — every reserved char encoded).
pub fn percent_encode_all(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// URL-encoded `next` value for the login redirect, or `""`. Only
/// same-origin paths outside the auth flow and `/api` are kept (query
/// preserved); dropped deep links fall back to the SPA's
/// `sessionStorage["hermes.lastLocation"]`.
///
/// PARITY: `_safe_next_target` (upstream lines 54-62).
pub fn safe_next_target(path: &str, query: &str) -> String {
    if path.is_empty() || !is_safe_next_path(path) {
        return String::new();
    }
    let target = if query.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{query}")
    };
    percent_encode_all(&target)
}

/// The login URL for unauthenticated requests: prefix + `/login` with
/// an optional encoded `next`.
pub fn login_url(prefix: &str, next_param: &str) -> String {
    if next_param.is_empty() {
        format!("{prefix}/login")
    } else {
        format!("{prefix}/login?next={next_param}")
    }
}

/// API routes → 401 JSON shape; HTML routes → 302 to `/login`.
/// fetch() follows a 302 opaquely into the cross-origin OAuth dance,
/// so API routes never get redirects; the SPA's 401 handler navigates
/// to `login_url` on `unauthenticated` / `session_expired`.
///
/// PARITY: `_unauth_response` (upstream lines 65-77).
#[derive(Debug, Clone, PartialEq)]
pub enum UnauthResponse {
    /// 401 JSON envelope for `/api/*` routes.
    Api {
        error: String,
        reason: String,
        login_url: String,
    },
    /// 302 redirect target for HTML routes.
    Redirect { location: String },
}

pub fn unauth_response(path: &str, prefix: &str, query: &str, reason: &str) -> UnauthResponse {
    let next_param = safe_next_target(path, query);
    let url = login_url(prefix, &next_param);
    if path.starts_with("/api/") {
        let expired = reason == "invalid_or_expired_session";
        UnauthResponse::Api {
            error: if expired {
                "session_expired".to_string()
            } else {
                "unauthenticated".to_string()
            },
            reason: reason.to_string(),
            login_url: url,
        }
    } else {
        UnauthResponse::Redirect { location: url }
    }
}

/// Auto-SSO decision inputs (explicit so the web surface supplies them).
pub struct SsoInputs<'a> {
    pub path: &'a str,
    /// Whether the one-shot loop-guard cookie is present.
    pub sso_attempt_present: bool,
    /// Names of registered session providers.
    pub session_providers: &'a [String],
    /// Whether the single provider is a password provider.
    pub single_is_password: bool,
    /// The single provider's name.
    pub single_name: &'a str,
}

/// 302 straight to `/auth/login` on an unauthenticated HTML load, or
/// `None`.
///
/// Only for a document load (not `/api/*`) when exactly one interactive
/// OAuth-style provider is registered (a password provider must render
/// the form) and the one-shot loop-guard cookie is absent — a present
/// marker means the portal had no session last time: clear it and fall
/// back to `/login` rather than ping-pong. Convenience, not a security
/// check.
///
/// PARITY: `_auto_sso_response` (upstream lines 80-107). The
/// `Some(ClearAndFallback)` arm carries the follow-up unauth response
/// the web surface must emit after clearing the marker.
#[derive(Debug, Clone, PartialEq)]
pub enum SsoDecision {
    /// Redirect to the provider's `/auth/login` (loop-guard must be set).
    Redirect { location: String },
    /// Marker present: clear it, then emit the inner unauth response.
    ClearAndFallback { fallback: Box<UnauthResponse> },
    /// No silent attempt (API route, or provider shape disallows it).
    None,
}

pub fn auto_sso_response(inputs: &SsoInputs<'_>, prefix: &str, query: &str) -> SsoDecision {
    if inputs.path.starts_with("/api/") {
        return SsoDecision::None;
    }
    if inputs.sso_attempt_present {
        let fallback = unauth_response(inputs.path, prefix, query, "no_cookie");
        return SsoDecision::ClearAndFallback {
            fallback: Box::new(fallback),
        };
    }
    if inputs.session_providers.len() != 1 || inputs.single_is_password {
        return SsoDecision::None;
    }
    let next_param = safe_next_target(inputs.path, query);
    let mut location = format!(
        "{prefix}/auth/login?provider={}",
        percent_encode_all(inputs.single_name)
    );
    if !next_param.is_empty() {
        location.push_str(&format!("&next={next_param}"));
    }
    SsoDecision::Redirect { location }
}

/// The gate's verdict for one request, in evaluation order. The web
/// surface performs the I/O each arm names.
///
/// PARITY: `gated_auth_middleware` (upstream lines 150-207) +
/// `_attempt_refresh` (lines 210-229) + `_serve_refreshed` (lines
/// 125-137) + `_session_expired_response` (lines 140-147).
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Gate disengaged (`auth_required` false) — pass through.
    PassThrough,
    /// Token-auth seam or public path already cleared it.
    AlreadyCleared,
    /// Verify this bearer against the provider stack (no cookies).
    VerifyBearer,
    /// Try the silent portal bounce before /login (no cookies at all).
    AttemptSso,
    /// Verify this access token (with optional provider hint).
    VerifySession { provider_hint: Option<String> },
    /// AT missing/failed but RT present: rotate via the coalesced
    /// refresh, then serve transparently on success.
    AttemptRefresh,
    /// Refresh failed (or no RT): 401/redirect + clear dead cookies.
    SessionExpired,
    /// Provider unreachable: 503, cookies kept (uncertain, not rejected).
    Unreachable,
    /// Verified session: serve, stamping the provider hint when absent.
    ServeWithSession { stamp_provider_hint: bool },
}

/// Classify one request into its gate verdict. Inputs are the already-read
/// request facts; `at_verified` / `refresh_outcome` are tri-state results
/// of the async verify/refresh steps the web surface runs between calls
/// (`None` = not yet attempted).
///
/// PARITY: the branch structure of `gated_auth_middleware`.
pub fn classify_request(
    auth_required: bool,
    token_authenticated: bool,
    path: &str,
    bearer: Option<&str>,
    access_token: Option<&str>,
    refresh_token: Option<&str>,
) -> GateDecision {
    if !auth_required {
        return GateDecision::PassThrough;
    }
    if token_authenticated || path_is_public(path) {
        return GateDecision::AlreadyCleared;
    }
    if bearer.is_some() {
        return GateDecision::VerifyBearer;
    }
    let has_at = access_token.is_some_and(|t| !t.is_empty());
    let has_rt = refresh_token.is_some_and(|t| !t.is_empty());
    if !has_at && !has_rt {
        return GateDecision::AttemptSso;
    }
    if has_at {
        return GateDecision::VerifySession {
            provider_hint: None,
        };
    }
    GateDecision::AttemptRefresh
}

/// Next step after the web surface attempted verification/refresh.
///
/// * `verified = Some(true)` → serve (stamp hint when the request had none).
/// * `verified = Some(false)`, RT present → refresh; no RT → expired.
/// * `verified = None` (unreachable raised) → 503, cookies kept.
/// * `refreshed = Some(true)` → serve with rotated cookies; `Some(false)`
///   → expired; `None` (unreachable) → 503.
pub fn next_after_verify(
    verified: Option<bool>,
    had_provider_hint: bool,
    session_provider: &str,
    has_refresh_token: bool,
    refreshed: Option<bool>,
) -> GateDecision {
    match verified {
        Some(true) => GateDecision::ServeWithSession {
            stamp_provider_hint: !had_provider_hint && !session_provider.is_empty(),
        },
        Some(false) => {
            if has_refresh_token {
                match refreshed {
                    Some(true) => GateDecision::ServeWithSession {
                        stamp_provider_hint: false,
                    },
                    Some(false) => GateDecision::SessionExpired,
                    None => GateDecision::Unreachable,
                }
            } else {
                GateDecision::SessionExpired
            }
        }
        None => GateDecision::Unreachable,
    }
}
