//! Cookie helpers for dashboard auth.
//!
//! PARITY: `hermes_cli/dashboard_auth/cookies.py` @ b9aa928 (whole
//! module). The FastAPI `Response`/`Request` seams become a
//! [`SetCookie`] directive list (the setter output) and a cookie-lookup
//! closure (the reader input); every name/attribute decision is identical.
//!
//! Three cookies in play:
//!   - `hermes_session_at`: the OAuth access token (HttpOnly, lifetime =
//!     token TTL, ~15 min)
//!   - `hermes_session_rt`: the OAuth refresh token (HttpOnly, lifetime =
//!     24h rotating upstream; RT cookie Max-Age is a generous 30-day upper
//!     bound — Portal's rotating RT TTL is the real authority). Written
//!     only when the provider returned a non-empty refresh token; a
//!     provider that omits it degrades to access-token-only sessions.
//!   - `hermes_session_pkce`: short-lived PKCE state + CSRF nonce +
//!     provider hint (HttpOnly, 10 minutes)
//!
//! All are SameSite=Lax (needed for the IDP redirect back to
//! /auth/callback) and live under the prefix's Path. `Secure` is set ONLY
//! when the dashboard was reached over HTTPS — loopback dev traffic is
//! always HTTP, so `Secure` would lock the cookies out of the browser.
//!
//! Cookie prefix selection (draft-west-cookie-prefixes hardening):
//!   * Loopback HTTP — bare name (`__Host-`/`__Secure-` require Secure).
//!   * Gated HTTPS, direct deploy (Path=/) — `__Host-` prefix (binds to
//!     the exact origin, no Domain attribute).
//!   * Gated HTTPS, behind a reverse-proxy prefix (Path=/hermes) —
//!     `__Secure-` prefix (`__Host-` is disallowed when Path != "/").
//!
//! The setters and readers BOTH consult the active prefix because the
//! cookie *name* changes — a reader that looked up the bare name when the
//! setter wrote `__Secure-hermes_session_at` would never find the value.

use serde_json::json;
use serde_json::Value;

// Bare cookie names — [`resolved_name`] decides whether to prepend
// `__Host-` / `__Secure-` based on the request's HTTPS + prefix
// combination.
pub const SESSION_AT_COOKIE: &str = "hermes_session_at";
pub const SESSION_RT_COOKIE: &str = "hermes_session_rt";
/// Non-secret provider routing hint: prevents a refresh token from being
/// handed to the wrong provider when several dashboard auth plugins are
/// enabled.
pub const SESSION_PROVIDER_COOKIE: &str = "hermes_session_provider";
pub const PKCE_COOKIE: &str = "hermes_session_pkce";
/// One-shot loop-guard marker for the auto-SSO redirect. Carries no secret
/// — it's a boolean breadcrumb — but is set HttpOnly/Lax/Secure like the
/// others for consistency. Short TTL so a user who returns later gets a
/// fresh silent attempt rather than a permanently-disabled one.
pub const SSO_ATTEMPT_COOKIE: &str = "hermes_sso_attempt";

/// Possible name variants to read back. Most-strict wins on iteration.
/// PARITY: `_NAME_VARIANTS` (upstream line 88).
pub const NAME_VARIANTS: [&str; 3] = ["__Host-", "__Secure-", ""];

/// RT cookie Max-Age: 30 days as a generous upper bound on the browser
/// lifetime; the upstream rotating-RT TTL (24h) is the real authority.
/// PARITY: `_RT_MAX_AGE` (upstream line 98).
pub const RT_MAX_AGE: i64 = 30 * 24 * 60 * 60;
/// PARITY: `_PKCE_MAX_AGE` (upstream line 99).
pub const PKCE_MAX_AGE: i64 = 10 * 60;
/// Auto-SSO loop-guard marker TTL: one redirect round trip, plus slack for
/// a slow portal hop or a manual back-button.
/// PARITY: `_SSO_ATTEMPT_MAX_AGE` (upstream line 106).
pub const SSO_ATTEMPT_MAX_AGE: i64 = 60;

/// One Set-Cookie directive (the transport-agnostic stand-in for
/// `response.set_cookie(...)`).
#[derive(Debug, Clone, PartialEq)]
pub struct SetCookie {
    pub name: String,
    pub value: String,
    pub max_age: i64,
    pub path: String,
    pub httponly: bool,
    pub samesite: String,
    pub secure: bool,
}

/// The request seam: look up a cookie value by (possibly prefixed) name.
pub type CookieLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Pick the cookie-prefix variant for the active request shape.
///
/// Mismatch between setter and reader would silently break sessions, so
/// this function is the single source of truth for naming.
///
/// PARITY: `_resolved_name` (upstream lines 114-128).
pub fn resolved_name(bare: &str, use_https: bool, prefix: &str) -> String {
    if !use_https {
        return bare.to_string();
    }
    if !prefix.is_empty() {
        // Path != "/" forbids __Host-; fall back to __Secure-.
        return format!("__Secure-{bare}");
    }
    format!("__Host-{bare}")
}

/// Cookie `Path` attribute for the active deploy shape: under a proxy
/// prefix the cookie is scoped to the prefix (browser omits it elsewhere,
/// and it doesn't leak to sibling apps on the same origin); direct-deploy
/// gets `Path=/`.
///
/// PARITY: `_cookie_path` (upstream lines 131-144).
pub fn cookie_path(prefix: &str) -> String {
    if prefix.is_empty() {
        "/".to_string()
    } else {
        prefix.to_string()
    }
}

/// PARITY: `_common_attrs` (upstream lines 147-154).
fn common_attrs(use_https: bool, prefix: &str) -> (String, bool, String, bool) {
    (cookie_path(prefix), true, "lax".to_string(), use_https)
}

#[allow(clippy::too_many_arguments)]
fn set_cookie(
    out: &mut Vec<SetCookie>,
    name: String,
    value: &str,
    max_age: i64,
    use_https: bool,
    prefix: &str,
) {
    let (path, httponly, samesite, secure) = common_attrs(use_https, prefix);
    out.push(SetCookie {
        name,
        value: value.to_string(),
        max_age,
        path,
        httponly,
        samesite,
        secure,
    });
}

/// Persist the non-secret provider routing hint for token refresh.
///
/// PARITY: `set_session_provider_cookie` (upstream lines 157-168).
pub fn set_session_provider_cookie(
    out: &mut Vec<SetCookie>,
    provider: &str,
    use_https: bool,
    prefix: &str,
) {
    if provider.is_empty() {
        return;
    }
    set_cookie(
        out,
        resolved_name(SESSION_PROVIDER_COOKIE, use_https, prefix),
        provider,
        RT_MAX_AGE,
        use_https,
        prefix,
    );
}

/// Set the session cookies on the response.
///
/// `access_token_expires_in` is in seconds (the provider's reported AT
/// TTL). `refresh_token` is written as the RT cookie when non-empty — a
/// provider that omits it (empty string) degrades to access-token-only
/// sessions; an empty-value RT cookie would be dead state at best, attack
/// surface at worst. `prefix` is the normalised X-Forwarded-Prefix value;
/// it influences both the cookie name and the Path attribute.
///
/// PARITY: `set_session_cookies` (upstream lines 171-217).
#[allow(clippy::too_many_arguments)]
pub fn set_session_cookies(
    out: &mut Vec<SetCookie>,
    access_token: &str,
    refresh_token: &str,
    access_token_expires_in: i64,
    use_https: bool,
    prefix: &str,
    provider: &str,
) {
    set_cookie(
        out,
        resolved_name(SESSION_AT_COOKIE, use_https, prefix),
        access_token,
        access_token_expires_in,
        use_https,
        prefix,
    );
    // Contract v1: empty refresh token means "don't persist RT cookie".
    if !refresh_token.is_empty() {
        set_cookie(
            out,
            resolved_name(SESSION_RT_COOKIE, use_https, prefix),
            refresh_token,
            RT_MAX_AGE,
            use_https,
            prefix,
        );
    }
    set_session_provider_cookie(out, provider, use_https, prefix);
}

/// Emit Max-Age=0 deletions for both session cookies.
///
/// The deletion's `Path` must match the set path AND the name must match
/// the variant the setter used; we don't know which variant fired, so we
/// emit deletions for every plausible variant under the active path.
///
/// PARITY: `clear_session_cookies` (upstream lines 220-241).
pub fn clear_session_cookies(out: &mut Vec<SetCookie>, prefix: &str) {
    let path = cookie_path(prefix);
    for variant in NAME_VARIANTS {
        for bare in [
            SESSION_AT_COOKIE,
            SESSION_RT_COOKIE,
            SESSION_PROVIDER_COOKIE,
        ] {
            out.push(SetCookie {
                name: format!("{variant}{bare}"),
                value: String::new(),
                max_age: 0,
                path: path.clone(),
                httponly: true,
                samesite: "lax".to_string(),
                secure: false,
            });
        }
    }
}

/// PARITY: `set_pkce_cookie` (upstream lines 244-251).
pub fn set_pkce_cookie(out: &mut Vec<SetCookie>, payload: &str, use_https: bool, prefix: &str) {
    set_cookie(
        out,
        resolved_name(PKCE_COOKIE, use_https, prefix),
        payload,
        PKCE_MAX_AGE,
        use_https,
        prefix,
    );
}

/// PARITY: `clear_pkce_cookie` (upstream lines 254-263).
pub fn clear_pkce_cookie(out: &mut Vec<SetCookie>, prefix: &str) {
    let path = cookie_path(prefix);
    for variant in NAME_VARIANTS {
        out.push(SetCookie {
            name: format!("{variant}{PKCE_COOKIE}"),
            value: String::new(),
            max_age: 0,
            path: path.clone(),
            httponly: true,
            samesite: "lax".to_string(),
            secure: false,
        });
    }
}

/// Read a cookie by checking every prefix variant in order — the request
/// that READS the cookie may not be the same shape as the request that SET
/// it; trying all three guarantees we find it.
///
/// PARITY: `_read_with_fallback` (upstream lines 266-280).
fn read_with_fallback(lookup: CookieLookup<'_>, bare_name: &str) -> Option<String> {
    for variant in NAME_VARIANTS {
        if let Some(value) = lookup(&format!("{variant}{bare_name}")) {
            return Some(value);
        }
    }
    None
}

/// Returns (access_token, refresh_token), either may be None.
///
/// PARITY: `read_session_cookies` (upstream lines 283-287).
pub fn read_session_cookies(lookup: CookieLookup<'_>) -> (Option<String>, Option<String>) {
    (
        read_with_fallback(lookup, SESSION_AT_COOKIE),
        read_with_fallback(lookup, SESSION_RT_COOKIE),
    )
}

/// Return the provider routing hint associated with the session cookies.
///
/// PARITY: `read_session_provider` (upstream lines 290-292).
pub fn read_session_provider(lookup: CookieLookup<'_>) -> Option<String> {
    read_with_fallback(lookup, SESSION_PROVIDER_COOKIE)
}

/// PARITY: `read_pkce_cookie` (upstream lines 295-297).
pub fn read_pkce_cookie(lookup: CookieLookup<'_>) -> Option<String> {
    read_with_fallback(lookup, PKCE_COOKIE)
}

/// Set the one-shot auto-SSO loop-guard marker (Phase 1). Value is a
/// constant `"1"` — only its presence matters.
///
/// PARITY: `set_sso_attempt_cookie` (upstream lines 300-312).
pub fn set_sso_attempt_cookie(out: &mut Vec<SetCookie>, use_https: bool, prefix: &str) {
    set_cookie(
        out,
        resolved_name(SSO_ATTEMPT_COOKIE, use_https, prefix),
        "1",
        SSO_ATTEMPT_MAX_AGE,
        use_https,
        prefix,
    );
}

/// Return the auto-SSO marker value if present (any variant), else None.
///
/// PARITY: `read_sso_attempt_cookie` (upstream lines 315-317).
pub fn read_sso_attempt_cookie(lookup: CookieLookup<'_>) -> Option<String> {
    read_with_fallback(lookup, SSO_ATTEMPT_COOKIE)
}

/// Emit Max-Age=0 deletions for the auto-SSO marker, every name variant.
/// Called on a successful callback and whenever the gate falls back to
/// /login.
///
/// PARITY: `clear_sso_attempt_cookie` (upstream lines 320-331).
pub fn clear_sso_attempt_cookie(out: &mut Vec<SetCookie>, prefix: &str) {
    let path = cookie_path(prefix);
    for variant in NAME_VARIANTS {
        out.push(SetCookie {
            name: format!("{variant}{SSO_ATTEMPT_COOKIE}"),
            value: String::new(),
            max_age: 0,
            path: path.clone(),
            httponly: true,
            samesite: "lax".to_string(),
            secure: false,
        });
    }
}

/// Decide whether to set the `Secure` cookie flag: true only when the
/// request URL scheme is https (which honours X-Forwarded-Proto under a
/// proxy-headers-enabled server).
///
/// PARITY: `detect_https` (upstream lines 334-341).
pub fn detect_https(request_scheme: Option<&str>) -> bool {
    request_scheme == Some("https")
}

/// JSON view helper for tests/logging (mirrors the attrs a Starlette
/// `set_cookie` call would carry).
pub fn set_cookie_to_json(cookie: &SetCookie) -> Value {
    json!({
        "name": cookie.name,
        "value": cookie.value,
        "max_age": cookie.max_age,
        "path": cookie.path,
        "httponly": cookie.httponly,
        "samesite": cookie.samesite,
        "secure": cookie.secure,
    })
}
