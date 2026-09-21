//! Parity tests for `hermes_cli/dashboard_auth/{public_paths,prefix}.py`
//! @ 5d59366. Prefix-normalisation and public-URL oracle cases mirror
//! `tests/hermes_cli/test_dashboard_auth_prefix.py` (the gate/cookie
//! HTTP cases belong to the web-server surface); env tests serialize
//! behind a mutex per the workspace convention.

use std::sync::Mutex;

use serde_json::json;

use hermes_cli::dashboard_auth::prefix::{
    normalise_prefix, resolve_public_url, resolve_public_url_with,
};
use hermes_cli::dashboard_auth::public_paths::{is_public_api_path, PUBLIC_API_PATHS};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── public_paths ─────────────────────────────────────────────────────────

#[test]
fn public_api_paths_membership() {
    // The liveness probes and read-only SPA feeds from the drift incident.
    for path in [
        "/api/health",
        "/api/status",
        "/api/config/defaults",
        "/api/config/schema",
        "/api/model/info",
        "/api/dashboard/themes",
        "/api/dashboard/plugins",
        // NAS cron-fire callback: the JWT, not this allowlist, is the
        // security boundary.
        "/api/cron/fire",
    ] {
        assert!(is_public_api_path(path), "{path}");
    }
    // Everything else gates.
    assert!(!is_public_api_path("/api/sessions"));
    assert!(!is_public_api_path("/api/config"));
    assert!(!is_public_api_path("/"));
    assert_eq!(PUBLIC_API_PATHS.len(), 8, "keep the allowlist minimal");
}

// ── normalise_prefix ─────────────────────────────────────────────────────

#[test]
fn prefix_normalisation_grammar() {
    assert_eq!(normalise_prefix(Some("/hermes")), "/hermes");
    assert_eq!(
        normalise_prefix(Some("hermes")),
        "/hermes",
        "leading slash added"
    );
    assert_eq!(
        normalise_prefix(Some("/hermes/")),
        "/hermes",
        "trailing slash removed"
    );
    assert_eq!(normalise_prefix(Some("  /hermes  ")), "/hermes", "trimmed");
    assert_eq!(normalise_prefix(None), "");
    assert_eq!(normalise_prefix(Some("")), "");
    assert_eq!(normalise_prefix(Some("   ")), "");
}

#[test]
fn prefix_injection_attempts_rejected() {
    for hostile in [
        "/a..b",
        "/a//b",
        "/hermes <script>",
        "/hermes\ttab",
        "/hermes\nnewline",
        "/her\"mes",
        "/her'mes",
    ] {
        assert_eq!(normalise_prefix(Some(hostile)), "", "hostile: {hostile:?}");
    }
    // Over-length prefixes rejected (HA ingress budgets the header size).
    let long = format!("/{}", "a".repeat(300));
    assert_eq!(normalise_prefix(Some(&long)), "");
}

// ── public URL resolution ────────────────────────────────────────────────

#[test]
fn public_url_requires_scheme_and_host() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL");
        std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "hermes.example.com");
    }
    // Missing scheme -> silently discarded (falls back to reconstruction).
    // The with-form resolves the config leg; with no config section the
    // result is "".
    assert_eq!(resolve_public_url_with(None), "");
    // With a config entry, the malformed env falls through to it.
    let dashboard = json!({"public_url": "https://hermes.example.com"});
    assert_eq!(
        resolve_public_url_with(Some(&dashboard)),
        "https://hermes.example.com"
    );
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}

#[test]
fn public_url_env_wins_over_config() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "https://env.example.com") };
    let dashboard = json!({"public_url": "https://config.example.com"});
    assert_eq!(
        resolve_public_url_with(Some(&dashboard)),
        "https://env.example.com",
        "env var has precedence"
    );
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}

#[test]
fn public_url_trailing_slash_stripped_and_hostile_rejected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "https://hermes.example.com/") };
    assert_eq!(resolve_public_url_with(None), "https://hermes.example.com");

    // Embedded newline / quotes: hard "no".
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "https://x.example.com/a b") };
    assert_eq!(resolve_public_url_with(None), "");
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}

#[test]
fn public_url_empty_env_falls_through_to_config() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // An empty env value is treated as unset so a provisioned-but-empty Fly
    // secret can't shadow a valid config entry.
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "") };
    let dashboard = json!({"public_url": "https://config.example.com"});
    assert_eq!(
        resolve_public_url_with(Some(&dashboard)),
        "https://config.example.com"
    );
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}

#[test]
fn resolve_without_config_is_env_only() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
    // config.yaml leg is the PENDING hermes_cli.config seam — env-only
    // resolution.
    assert_eq!(resolve_public_url(), "");
}

/// PARITY: `TestForwardedPrefixNormalisation::
/// test_home_assistant_ingress_prefix_with_subpath_is_accepted` — HA
/// Supervisor ingress prefixes are 63+ chars before add-ons append
/// their own mount path; they must survive validation.
#[test]
fn ha_ingress_prefix_with_subpath_accepted() {
    let prefix = "/api/hassio_ingress/8AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AbCdEf/dashboard";
    assert!(prefix.len() > 64);
    assert_eq!(normalise_prefix(Some(prefix)), prefix);
}

/// PARITY: `TestForwardedPrefixNormalisation::
/// test_overlong_prefix_is_rejected_with_deduplicated_warning` — the
/// return shape (dedup is log-side, asserted upstream via caplog).
#[test]
fn overlong_prefix_rejected() {
    let too_long = format!("/{}", "a".repeat(257));
    for _ in 0..3 {
        assert_eq!(normalise_prefix(Some(&too_long)), "");
    }
}

/// PARITY: `urllib.parse.urlparse` rejects malformed IPv6 (`ValueError`
/// → `""`). A public URL with an unmatched bracket is malformed, not
/// a host.
#[test]
fn public_url_malformed_ipv6_rejected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "http://[::1") };
    assert_eq!(resolve_public_url_with(None), "");
    // Matched brackets are a valid host and survive.
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "http://[::1]/x") };
    assert_eq!(resolve_public_url_with(None), "http://[::1]/x");
    // Brackets rejected exactly like urlparse: stray closers, stray
    // openers, empty/single-char hosts; userinfo + IPv6 is fine.
    for bad in [
        "http://example.com]/x",
        "http://exa[mple.com/",
        "http://[]/x",
        "http://[a]/x",
    ] {
        unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", bad) };
        assert_eq!(resolve_public_url_with(None), "", "reject {bad}");
    }
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "http://user@[::1]/") };
    assert_eq!(resolve_public_url_with(None), "http://user@[::1]");
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}

/// PARITY: scheme comparison is case-insensitive (urlparse lowercases
/// the scheme) while the netloc keeps its case.
#[test]
fn public_url_scheme_case_insensitive() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("HERMES_DASHBOARD_PUBLIC_URL", "HTTP://EXAMPLE.COM/x") };
    assert_eq!(resolve_public_url_with(None), "HTTP://EXAMPLE.COM/x");
    unsafe { std::env::remove_var("HERMES_DASHBOARD_PUBLIC_URL") };
}
