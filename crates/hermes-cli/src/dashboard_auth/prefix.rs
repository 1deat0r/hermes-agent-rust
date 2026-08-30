//! Helpers for X-Forwarded-Prefix support and the operator-declared
//! public URL.
//!
//! PARITY: `hermes_cli/dashboard_auth/prefix.py` @ b9aa928 (whole module).
//!
//! Mission-control style deploys reverse-proxy the dashboard at a path
//! prefix, injecting `X-Forwarded-Prefix: /hermes` so the backend can
//! reconstruct prefixed URLs (Location headers, OAuth redirect_uri, cookie
//! Path attributes, SPA asset URLs). This module is also the home of the
//! `HERMES_DASHBOARD_PUBLIC_URL` / `dashboard.public_url` resolution — a
//! complete public URL (scheme + host + optional path prefix) is used
//! directly for the OAuth redirect_uri, skipping prefix reconstruction.
//!
//! TRANSLATION NOTE: `_load_dashboard_section` delegates to
//! `hermes_cli.config.load_config` upstream; that surface has not ported,
//! so [`resolve_public_url`] resolves env-only and
//! [`resolve_public_url_with`] accepts the parsed `dashboard` section for
//! the config.yaml leg (PENDING seam).

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Mutex;

/// Home Assistant Supervisor ingress prefixes are already 63 chars before
/// deployments add their own sub-path. Bounded header budget, with room
/// for mainstream reverse-proxy path mounts.
///
/// PARITY: `_MAX_PREFIX_LENGTH` (upstream line 26).
const MAX_PREFIX_LENGTH: usize = 256;

/// Characters that indicate a typo or a header-injection attempt; the
/// whole value is rejected rather than sanitised.
/// PARITY: `_REJECT_CHARS` (upstream line 29).
const REJECT_CHARS: [char; 8] = ['"', '\'', '<', '>', ' ', '\n', '\r', '\t'];

/// Which (source, value) pairs we've already warned about —
/// `resolve_public_url` runs per request, so an un-deduplicated warning
/// would flood the logs for a misconfigured deploy.
/// PARITY: `_warned_malformed_public_urls` / `_warned_malformed_prefixes`.
static WARNED_MALFORMED_PUBLIC_URLS: Lazy<Mutex<HashSet<(String, String)>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));
static WARNED_MALFORMED_PREFIXES: Lazy<Mutex<HashSet<(String, String)>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// Warn (once per distinct value) when a non-empty public-url value was
/// rejected — almost always a missing scheme, the single most common
/// cause of "I set HERMES_DASHBOARD_PUBLIC_URL but the OAuth callback is
/// still http://".
///
/// PARITY: `_warn_if_malformed` (upstream lines 44-73); the message text
/// is preserved verbatim so operator greps still match.
fn warn_if_malformed(source: &str, raw: &str) {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return; // empty/unset is a legitimate "no override" — not malformed
    }
    let key = (source.to_string(), cleaned.to_string());
    {
        let mut warned = WARNED_MALFORMED_PUBLIC_URLS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if warned.contains(&key) {
            return;
        }
        warned.insert(key);
    }
    let host_hint = cleaned.rsplit("://").next().unwrap_or("hermes.example.com");
    let host_hint = if host_hint.is_empty() {
        "hermes.example.com"
    } else {
        host_hint
    };
    log::warn!(
        "{source} is set to {cleaned:?} but was ignored because it is not a valid \
         absolute URL — it must include an http:// or https:// scheme \
         (e.g. https://{host_hint}). Falling back to reconstructing the OAuth \
         redirect URI from request headers, which may produce the wrong scheme \
         behind a reverse proxy."
    );
}

/// Warn once when a non-empty X-Forwarded-Prefix value is rejected.
///
/// PARITY: `_warn_if_malformed_prefix` (upstream lines 76-89).
fn warn_if_malformed_prefix(raw: &str, reason: &str) {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return;
    }
    let key = (cleaned.to_string(), reason.to_string());
    {
        let mut warned = WARNED_MALFORMED_PREFIXES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if warned.contains(&key) {
            return;
        }
        warned.insert(key);
    }
    log::warn!(
        "X-Forwarded-Prefix header {cleaned:?} was ignored because {reason}. \
         Dashboard URLs will be generated without a reverse-proxy path prefix."
    );
}

/// Normalise an X-Forwarded-Prefix header value.
///
/// Returns `"/hermes"` (no trailing slash) or `""` when no prefix is set /
/// the header is malformed. Anything containing `..` or disallowed
/// characters is rejected so a hostile proxy can't inject HTML or
/// path-traversal sequences via the prefix.
///
/// PARITY: `normalise_prefix` (upstream lines 92-124).
pub fn normalise_prefix(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return String::new();
    };
    let mut p = raw.trim().to_string();
    if p.is_empty() {
        return String::new();
    }
    if !p.starts_with('/') {
        p = format!("/{p}");
    }
    while p.ends_with('/') {
        p.pop();
    }
    if p.contains("//") || p.contains("..") || p.chars().any(|c| REJECT_CHARS.contains(&c)) {
        warn_if_malformed_prefix(raw, "it contains a disallowed character or path sequence");
        return String::new();
    }
    if p.len() > MAX_PREFIX_LENGTH {
        warn_if_malformed_prefix(
            raw,
            &format!("it is longer than {MAX_PREFIX_LENGTH} characters"),
        );
        return String::new();
    }
    p
}

/// Normalise a `dashboard.public_url` value.
///
/// Returns the cleaned URL (scheme://netloc[/path], trailing slash
/// removed) on success, or `""` when the value is empty, malformed, or
/// contains header-injection characters. The caller must treat `""` as
/// "fall back to request reconstruction" — never as "the user explicitly
/// chose no public URL".
///
/// PARITY: `_normalise_public_url` (upstream lines 128-166).
fn normalise_public_url(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return String::new();
    };
    let url = raw.trim();
    if url.is_empty() {
        return String::new();
    }
    // Hard "no" on control/quote/whitespace characters — urlparse is
    // permissive enough to accept some hostile values.
    if url.chars().any(|c| REJECT_CHARS.contains(&c)) {
        return String::new();
    }
    // `urllib.parse.urlparse` scheme/netloc check.
    let (scheme, rest) = match url.split_once("://") {
        Some((scheme, rest)) => (scheme.to_lowercase(), rest),
        None => return String::new(),
    };
    if scheme != "http" && scheme != "https" {
        return String::new();
    }
    let netloc = rest.split(['/', '?', '#']).next().unwrap_or("");
    if netloc.is_empty() {
        return String::new();
    }
    // Strip a single trailing slash so callers can append paths without
    // producing `//` double-slashes.
    url.trim_end_matches('/').to_string()
}

/// Resolve the operator-declared dashboard public URL from the parsed
/// `dashboard` config section (the `hermes_cli.config` seam is parameterised
/// here). See [`resolve_public_url`] for the precedence contract.
pub fn resolve_public_url_with(dashboard_section: Option<&Value>) -> String {
    let env_raw = std::env::var("HERMES_DASHBOARD_PUBLIC_URL").unwrap_or_default();
    let env_clean = normalise_public_url(Some(&env_raw));
    if !env_clean.is_empty() {
        return env_clean;
    }
    warn_if_malformed("HERMES_DASHBOARD_PUBLIC_URL env var", &env_raw);
    let cfg_raw = dashboard_section
        .and_then(|section| section.get("public_url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let cfg_clean = normalise_public_url(Some(&cfg_raw));
    if cfg_clean.is_empty() {
        warn_if_malformed("dashboard.public_url in config.yaml", &cfg_raw);
    }
    cfg_clean
}

/// Resolve the operator-declared dashboard public URL.
///
/// Precedence:
///   1. `HERMES_DASHBOARD_PUBLIC_URL` env var (empty values are unset — a
///      provisioned-but-unpopulated Fly secret can't shadow config.yaml).
///   2. `dashboard.public_url` in config.yaml.
///   3. `""` — "no override, reconstruct from request".
///
/// A malformed env var falls through to the config.yaml entry; a malformed
/// config entry falls through to `""` — a typo in one surface doesn't
/// prevent the other from working.
///
/// PARITY: `resolve_public_url` (upstream lines 190-221), with the
/// config.yaml leg currently resolving to empty (PENDING
/// `hermes_cli.config`).
pub fn resolve_public_url() -> String {
    resolve_public_url_with(None)
}
