//! Pure decision helpers behind the dashboard auth routes.
//!
//! PARITY: the transport-neutral functions in
//! `hermes_cli/dashboard_auth/routes.py` @ 5d59366. The `@router`
//! handlers themselves (Request/Response wiring, redirects, cookie I/O)
//! belong to the web-server surface; every *decision* they make ports
//! here over explicit inputs.
//!
//! Covered: `_validate_post_login_target` (lines 92-97),
//! `_validate_loopback_redirect_uri` (lines 218-229),
//! `_select_native_provider` (lines 232-239), the password rate limiter
//! (lines 338-361), `_bearer_payload` (lines 110-115),
//! `_provider_pkce_segments` (lines 85-89), `_redirect_uri` helpful
//! assembly (lines 69-83), and the native-finish redirect assembly
//! (lines 118-133).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use once_cell::sync::Lazy;

use super::base::Session;
use super::request_utils::is_safe_next_path;

/// `raw` (URL-decoded) if it is a safe same-origin path, else `""`.
/// Re-validated at every hop because a `next=` value can re-enter via
/// a crafted URL.
///
/// PARITY: `_validate_post_login_target` (upstream lines 92-97).
pub fn validate_post_login_target(decoded_raw: &str) -> String {
    if decoded_raw.is_empty() || !is_safe_next_path(decoded_raw) {
        return String::new();
    }
    decoded_raw.to_string()
}

/// Accept only `http://127.0.0.1[:port]/…` / `http://[::1][:port]/…`.
/// Security boundary: the route is public, so a non-loopback host
/// would make the callback an open redirect leaking a live code.
/// `localhost` is rejected (RFC 8252 §8.3).
///
/// PARITY: `_validate_loopback_redirect_uri` (upstream lines 218-229).
#[derive(Debug, Clone, PartialEq)]
pub enum LoopbackError {
    Missing,
    NotHttp,
    NotLoopback,
}

impl std::fmt::Display for LoopbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopbackError::Missing => write!(f, "redirect_uri required"),
            LoopbackError::NotHttp => write!(
                f,
                "native redirect_uri must be http:// on the loopback interface"
            ),
            LoopbackError::NotLoopback => write!(
                f,
                "native redirect_uri host must be a loopback IP literal (127.0.0.1 / ::1)"
            ),
        }
    }
}

pub fn validate_loopback_redirect_uri(raw: &str) -> Result<String, LoopbackError> {
    if raw.is_empty() {
        return Err(LoopbackError::Missing);
    }
    let rest = raw.strip_prefix("http://").ok_or(LoopbackError::NotHttp)?;
    let host = if let Some(stripped) = rest.strip_prefix('[') {
        // Bracketed IPv6: host runs to `]`.
        let end = stripped.find(']').ok_or(LoopbackError::NotLoopback)?;
        let (host, after) = (&stripped[..end], &stripped[end + 1..]);
        if !after.is_empty() && !after.starts_with(':') && !after.starts_with('/') {
            return Err(LoopbackError::NotLoopback);
        }
        host.to_string()
    } else {
        rest.split(['/', '?', '#', ':'])
            .next()
            .unwrap_or("")
            .to_string()
    };
    if host.to_lowercase() != "127.0.0.1" && host.to_lowercase() != "::1" {
        return Err(LoopbackError::NotLoopback);
    }
    Ok(raw.to_string())
}

/// Resolve the provider for a native authorize request. An empty
/// `provider` auto-selects the ONLY interactive session provider (with
/// several the caller renders a chooser instead of guessing).
/// Returns the selected name, or `None` when the caller must choose.
///
/// PARITY: `_select_native_provider` (upstream lines 232-239).
/// `explicit` is `get_provider(provider)`; `session_providers` is
/// `list_session_providers()` — both supplied by the web surface.
pub fn select_native_provider(
    requested: &str,
    explicit: Option<String>,
    session_providers: &[String],
) -> Option<String> {
    if !requested.is_empty() {
        return explicit;
    }
    if session_providers.len() == 1 {
        return Some(session_providers[0].clone());
    }
    None
}

/// Password-login rate limiter: 10 attempts per 60 s sliding window per
/// IP. An empty IP shares one bucket — fail-safe toward throttling.
///
/// PARITY: `_password_rate_limited` (upstream lines 344-356).
pub const PASSWORD_RATE_MAX_ATTEMPTS: usize = 10;
pub const PASSWORD_RATE_WINDOW_SECS: f64 = 60.0;

static PASSWORD_ATTEMPTS: Lazy<Mutex<HashMap<String, Vec<Instant>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn password_rate_limited(ip: &str) -> bool {
    password_rate_limited_at(ip, Instant::now())
}

fn password_rate_limited_at(ip: &str, now: Instant) -> bool {
    let key = if ip.is_empty() { "_unknown_" } else { ip }.to_string();
    let mut attempts = PASSWORD_ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    let bucket = attempts.entry(key).or_default();
    let window = std::time::Duration::from_secs_f64(PASSWORD_RATE_WINDOW_SECS);
    bucket.retain(|t| *t + window > now);
    if bucket.len() >= PASSWORD_RATE_MAX_ATTEMPTS {
        return true;
    }
    bucket.push(now);
    false
}

/// Test-only: clear all rate-limit buckets.
///
/// PARITY: `_reset_password_rate_limit` (upstream lines 359-361).
pub fn reset_password_rate_limit() {
    PASSWORD_ATTEMPTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// JSON body for the native token/refresh endpoints (tokens in body,
/// no cookie).
///
/// PARITY: `_bearer_payload` (upstream lines 110-115).
pub fn bearer_payload(session: &Session) -> serde_json::Value {
    serde_json::json!({
        "access_token": session.access_token,
        "refresh_token": session.refresh_token,
        "token_type": "Bearer",
        "expires_at": session.expires_at,
        "provider": session.provider,
        "user_id": session.user_id,
    })
}

/// Parse a provider's flat `state=…;verifier=…` PKCE string into a dict
/// — the ONE place the flat form is parsed.
///
/// PARITY: `_provider_pkce_segments` (upstream lines 85-89).
pub fn provider_pkce_segments(cookie_payload: &HashMap<String, String>) -> HashMap<String, String> {
    let flat = cookie_payload
        .get("hermes_session_pkce")
        .map(String::as_str)
        .unwrap_or("");
    flat.split(';')
        .filter_map(|seg| {
            seg.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

/// Absolute `/auth/callback` URL handed to the IDP. An operator-declared
/// public URL is the complete authority (`X-Forwarded-Prefix` ignored so
/// a baked-in prefix is not doubled); otherwise the app URL (which
/// honours `X-Forwarded-Host/Proto`) with the prefix prepended.
///
/// PARITY: `_redirect_uri` (upstream lines 69-83). `app_callback_url`
/// is `str(request.url_for("auth_callback"))`, already an absolute URL.
pub fn redirect_uri(public_url: &str, app_callback_url: &str, prefix: &str) -> String {
    if !public_url.is_empty() {
        return format!("{}/auth/callback", public_url.trim_end_matches('/'));
    }
    if prefix.is_empty() {
        return app_callback_url.to_string();
    }
    // Prepend the prefix to the URL path (Starlette does not do this).
    match app_callback_url.split_once("://") {
        Some((scheme, rest)) => match rest.find('/') {
            Some(idx) => {
                let (authority, path) = rest.split_at(idx);
                format!("{scheme}://{authority}{prefix}{path}")
            }
            None => format!("{app_callback_url}{prefix}"),
        },
        None => format!("{prefix}{app_callback_url}"),
    }
}

/// Assemble the desktop's `redirect_uri?code=…&state=…` after minting
/// the one-time loopback code. No session cookies on the native path.
///
/// PARITY: `_finish_native_login` URL assembly (upstream lines 118-133;
/// the store calls and audits are the web surface's).
pub fn native_finish_url(redirect_uri: &str, gw_code: &str, client_state: &str) -> String {
    let sep = if redirect_uri.contains('?') { "&" } else { "?" };
    format!(
        "{redirect_uri}{sep}code={}&state={}",
        percent_encode(gw_code),
        percent_encode(client_state)
    )
}

fn percent_encode(raw: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_window_slides() {
        reset_password_rate_limit();
        for _ in 0..PASSWORD_RATE_MAX_ATTEMPTS {
            assert!(!password_rate_limited("9.9.9.9"));
        }
        assert!(password_rate_limited("9.9.9.9"));
        reset_password_rate_limit();
        assert!(!password_rate_limited("9.9.9.9"));
    }
}
