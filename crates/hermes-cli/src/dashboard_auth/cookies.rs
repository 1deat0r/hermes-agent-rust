//! Cookie helpers for dashboard auth.
//!
//! PARITY: `hermes_cli/dashboard_auth/cookies.py` @ 5d59366 (whole
//! module, 222 lines). The FastAPI `Response`/`Request` seams become a
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
/// PARITY: `_NAME_VARIANTS` (upstream line 34).
pub const NAME_VARIANTS: [&str; 3] = ["__Host-", "__Secure-", ""];

/// RT cookie Max-Age: 30 days as a generous upper bound on the browser
/// lifetime; the upstream rotating-RT TTL (24h) is the real authority.
/// PARITY: `_RT_MAX_AGE` (upstream line 38).
pub const RT_MAX_AGE: i64 = 30 * 24 * 60 * 60;
/// PARITY: `_PKCE_MAX_AGE` (upstream line 39).
pub const PKCE_MAX_AGE: i64 = 10 * 60;
/// Auto-SSO loop-guard marker TTL: one redirect round trip, plus slack for
/// a slow portal hop or a manual back-button.
/// PARITY: `_SSO_ATTEMPT_MAX_AGE` (upstream line 42).
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
/// PARITY: `_resolved_name` (upstream lines 47-51).
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
/// PARITY: `_cookie_path` (upstream lines 54-56).
pub fn cookie_path(prefix: &str) -> String {
    if prefix.is_empty() {
        "/".to_string()
    } else {
        prefix.to_string()
    }
}

/// PARITY: `_common_attrs` (upstream lines 59-63).
fn common_attrs(use_https: bool, prefix: &str) -> (String, bool, String, bool) {
    (cookie_path(prefix), true, "lax".to_string(), use_https)
}

/// Attributes shared by the PKCE set AND clear paths — a shape mismatch
/// means the browser silently keeps the stale cookie. SameSite=None over
/// HTTPS (the PKCE cookie is set on the /auth/login 302 and must survive
/// the cross-site IDP redirect chain; Chromium drops Lax cookies set on
/// such a 302, crbug 40508226); Lax without Secure over HTTP (None
/// requires Secure, which HTTP cannot carry).
///
/// PARITY: `_pkce_attrs` (upstream lines 66-72).
fn pkce_attrs(use_https: bool, prefix: &str) -> (String, bool, String, bool) {
    if use_https {
        (cookie_path(prefix), true, "none".to_string(), true)
    } else {
        (cookie_path(prefix), true, "lax".to_string(), false)
    }
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

/// Push one Set-Cookie directive with explicit attributes (the PKCE
/// setter, whose shape differs from the common one).
fn set_cookie_with(
    out: &mut Vec<SetCookie>,
    name: String,
    value: &str,
    max_age: i64,
    path: String,
    httponly: bool,
    samesite: &str,
    secure: bool,
) {
    out.push(SetCookie {
        name,
        value: value.to_string(),
        max_age,
        path,
        httponly,
        samesite: samesite.to_string(),
        secure,
    });
}

/// Emit Max-Age=0 deletions for every plausible name variant (the
/// setting request's shape is unknown). Prefixed names are rejected by
/// the browser unless they carry `Secure` (`__Host-` additionally
/// requires `Path=/`), so those deletions always do; the bare deletion
/// mirrors the setter's shape via `bare_samesite`/`bare_secure`, which
/// works on both HTTP and HTTPS origins.
///
/// PARITY: `_clear_cookie_variants` (upstream lines 109-120).
fn clear_cookie_variants(
    out: &mut Vec<SetCookie>,
    bare_name: &str,
    prefix: &str,
    https_samesite: &str,
    bare_samesite: &str,
    bare_secure: bool,
) {
    for (variant, path) in [
        ("__Host-".to_string(), "/".to_string()),
        ("__Secure-".to_string(), cookie_path(prefix)),
    ] {
        set_cookie_with(
            out,
            format!("{variant}{bare_name}"),
            "",
            0,
            path,
            true,
            https_samesite,
            true,
        );
    }
    set_cookie_with(
        out,
        bare_name.to_string(),
        "",
        0,
        cookie_path(prefix),
        true,
        bare_samesite,
        bare_secure,
    );
}

/// Persist the non-secret provider routing hint for token refresh.
///
/// PARITY: `set_session_provider_cookie` (upstream lines 82-87).
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
/// PARITY: `set_session_cookies` (upstream lines 90-106).
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

/// Emit Max-Age=0 deletions for the AT, RT and provider cookies
/// (every name variant, active path).
///
/// PARITY: `clear_session_cookies` (upstream lines 123-128). The bare
/// deletion mirrors the session setter (Lax, no Secure) so it still
/// works on plain-HTTP origins; the prefixed deletions always carry
/// Secure (browsers reject prefixed Set-Cookie otherwise, and the
/// session would survive logout on HTTPS origins).
pub fn clear_session_cookies(out: &mut Vec<SetCookie>, prefix: &str) {
    for bare in [
        SESSION_AT_COOKIE,
        SESSION_RT_COOKIE,
        SESSION_PROVIDER_COOKIE,
    ] {
        clear_cookie_variants(out, bare, prefix, "lax", "lax", false);
    }
}

/// Set the PKCE cookie (payload segment dict, encoded via
/// [`encode_pkce_payload`]).
///
/// PARITY: `set_pkce_cookie` (upstream lines 140-144).
pub fn set_pkce_cookie(
    out: &mut Vec<SetCookie>,
    payload: &std::collections::HashMap<String, String>,
    use_https: bool,
    prefix: &str,
) {
    let (path, httponly, samesite, secure) = pkce_attrs(use_https, prefix);
    set_cookie_with(
        out,
        resolved_name(PKCE_COOKIE, use_https, prefix),
        &encode_pkce_payload(payload),
        PKCE_MAX_AGE,
        path,
        httponly,
        &samesite,
        secure,
    );
}

/// Delete every PKCE cookie variant (prefixed ones carry
/// `Secure; SameSite=None`, matching the HTTPS setter so the browser
/// honours the deletion for whichever variant was actually set).
///
/// PARITY: `clear_pkce_cookie` (upstream lines 147-151).
pub fn clear_pkce_cookie(out: &mut Vec<SetCookie>, use_https: bool, prefix: &str) {
    let (_, _, bare_samesite, bare_secure) = pkce_attrs(use_https, prefix);
    clear_cookie_variants(
        out,
        PKCE_COOKIE,
        prefix,
        "none",
        &bare_samesite,
        bare_secure,
    );
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
/// PARITY: `clear_sso_attempt_cookie` (upstream lines 213-217).
pub fn clear_sso_attempt_cookie(out: &mut Vec<SetCookie>, prefix: &str) {
    for bare in [SSO_ATTEMPT_COOKIE] {
        clear_cookie_variants(out, bare, prefix, "lax", "lax", false);
    }
}

/// Decide whether to set the `Secure` cookie flag: true only when the
/// request URL scheme is https (which honours X-Forwarded-Proto under a
/// proxy-headers-enabled server).
///
/// PARITY: `detect_https` (upstream lines 220-222).
pub fn detect_https(request_scheme: Option<&str>) -> bool {
    request_scheme == Some("https")
}

/// Wire value `base64url(JSON)`, no padding. The urlsafe alphabet is a
/// strict subset of RFC 6265 cookie-octets, so strict proxies (Go
/// net/http) never quote it; padding `=` would trigger quoting, the
/// parser restores it.
///
/// PARITY: `encode_pkce_payload` (upstream lines 131-137). Keys sort
/// via `BTreeMap` (`sort_keys=True`); `serde_json::to_string` emits the
/// compact `,`/`:` separators.
pub fn encode_pkce_payload(parts: &std::collections::HashMap<String, String>) -> String {
    let ordered: std::collections::BTreeMap<&String, &String> = parts.iter().collect();
    let raw = serde_json::to_string(&ordered).expect("segment map serializes");
    base64_urlsafe_nopad(raw.as_bytes())
}

fn base64_urlsafe_nopad(input: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

/// Inverse of [`encode_pkce_payload`]. EVERY reader must go through
/// this — reading the raw wire value parses zero segments and silently
/// disables the check it feeds. Compatibility ladder for cookies minted
/// by an older server mid-upgrade: 1. base64url(JSON); 2. flat form with
/// raw `;` delimiters, split WITHOUT unquoting (the `next` segment
/// carries its own URL-encoding); 3. URL-encoded flat form, unquote once
/// then split.
///
/// PARITY: `parse_pkce_payload` (upstream lines 176-199).
pub fn parse_pkce_payload(raw: &str) -> std::collections::HashMap<String, String> {
    // Rung 1: base64url(JSON). Legacy forms always contain `%` or `;`
    // (outside the urlsafe alphabet) so they can never match.
    if !raw.is_empty() && raw.chars().all(is_b64url_char) {
        let padded = format!("{raw}{}", "=".repeat((4 - raw.len() % 4) % 4));
        if let Ok(decoded) = base64_urlsafe_decode(&padded) {
            if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&decoded) {
                return map
                    .into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                    .collect();
            }
        }
    }
    // Rungs 2/3: flat form. Raw `;` present → split as-is (never unquote
    // first: a `%3B` inside `next` would become a bogus delimiter);
    // otherwise unquote once then split.
    let flat = if raw.contains(';') {
        raw.to_string()
    } else {
        percent_decode(raw)
    };
    flat.split(';')
        .filter_map(|seg| {
            seg.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

fn is_b64url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn base64_urlsafe_decode(padded: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE
        .decode(padded)
        .map_err(|e| e.to_string())
}

/// Single-pass `%XX` decoder (the `unquote` rung of the compat ladder).
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 {
            if let (Some(h), Some(l)) = (
                hex_val(bytes.get(i + 1).copied().unwrap_or(0)),
                hex_val(bytes.get(i + 2).copied().unwrap_or(0)),
            ) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
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
