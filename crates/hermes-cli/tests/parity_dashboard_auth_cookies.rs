//! Parity tests for `hermes_cli/dashboard_auth/cookies.py` @ b9aa928.
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use std::collections::HashMap;

use hermes_cli::dashboard_auth::cookies::{
    clear_pkce_cookie, clear_session_cookies, clear_sso_attempt_cookie, cookie_path, detect_https,
    read_pkce_cookie, read_session_cookies, read_session_provider, resolved_name, set_pkce_cookie,
    set_session_cookies, set_session_provider_cookie, set_sso_attempt_cookie, NAME_VARIANTS,
    PKCE_MAX_AGE, RT_MAX_AGE, SSO_ATTEMPT_MAX_AGE,
};

fn lookup_from(map: &HashMap<String, String>) -> impl Fn(&str) -> Option<String> + '_ {
    move |name: &str| map.get(name).cloned()
}

#[test]
fn resolved_name_follows_the_three_prefix_rules() {
    // Loopback HTTP: bare name (__Host-/__Secure- require Secure).
    assert_eq!(
        resolved_name("hermes_session_at", false, ""),
        "hermes_session_at"
    );
    assert_eq!(
        resolved_name("hermes_session_at", false, "/hermes"),
        "hermes_session_at"
    );
    // Gated HTTPS direct deploy: __Host-.
    assert_eq!(
        resolved_name("hermes_session_at", true, ""),
        "__Host-hermes_session_at"
    );
    // Gated HTTPS behind a prefix: __Secure- (Path != "/" forbids __Host-).
    assert_eq!(
        resolved_name("hermes_session_at", true, "/hermes"),
        "__Secure-hermes_session_at"
    );
}

#[test]
fn cookie_path_follows_the_prefix() {
    assert_eq!(cookie_path(""), "/");
    assert_eq!(cookie_path("/hermes"), "/hermes");
}

#[test]
fn set_session_cookies_full_shape_https_direct_deploy() {
    let mut out = Vec::new();
    set_session_cookies(&mut out, "at-value", "rt-value", 900, true, "", "nous");
    assert_eq!(out.len(), 3, "AT + RT + provider routing hint");
    let at = &out[0];
    assert_eq!(at.name, "__Host-hermes_session_at");
    assert_eq!(at.value, "at-value");
    assert_eq!(at.max_age, 900, "provider-reported AT TTL");
    assert_eq!(at.path, "/");
    assert!(at.httponly && at.secure);
    assert_eq!(at.samesite, "lax");
    let rt = &out[1];
    assert_eq!(rt.name, "__Host-hermes_session_rt");
    assert_eq!(rt.value, "rt-value");
    assert_eq!(rt.max_age, RT_MAX_AGE);
    let provider = &out[2];
    assert_eq!(provider.name, "__Host-hermes_session_provider");
    assert_eq!(provider.value, "nous");
    assert_eq!(provider.max_age, RT_MAX_AGE);
}

#[test]
fn set_session_cookies_over_http_drops_secure_and_prefix() {
    let mut out = Vec::new();
    set_session_cookies(&mut out, "at", "", 900, false, "", "");
    assert_eq!(out[0].name, "hermes_session_at");
    assert!(
        !out[0].secure,
        "loopback is HTTP; Secure would lock cookies out"
    );
}

#[test]
fn empty_refresh_token_degrades_to_access_token_only() {
    let mut out = Vec::new();
    set_session_cookies(&mut out, "at", "", 900, true, "", "nous");
    // Contract v1: empty RT means don't persist the RT cookie. AT + provider
    // hint only.
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|c| !c.name.contains("hermes_session_rt")));
}

#[test]
fn clear_session_cookies_emits_all_variants_both_cookies() {
    let mut out = Vec::new();
    clear_session_cookies(&mut out, "/hermes");
    // 3 name variants x (AT + RT + provider).
    assert_eq!(out.len(), 9);
    for cookie in &out {
        assert_eq!(cookie.value, "");
        assert_eq!(cookie.max_age, 0, "Max-Age=0 deletion");
        assert_eq!(
            cookie.path, "/hermes",
            "deletion Path must match the set path"
        );
        assert!(cookie.httponly);
        assert_eq!(cookie.samesite, "lax");
    }
    let names: Vec<&str> = out.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"__Host-hermes_session_at"));
    assert!(names.contains(&"__Secure-hermes_session_rt"));
    assert!(names.contains(&"hermes_session_provider"));
}

#[test]
fn read_fallback_finds_every_variant_most_strict_first() {
    let mut jar = HashMap::new();
    jar.insert("__Host-hermes_session_at".to_string(), "host".to_string());
    jar.insert(
        "__Secure-hermes_session_at".to_string(),
        "secure".to_string(),
    );
    jar.insert("__Secure-hermes_session_rt".to_string(), "rt".to_string());
    let lookup = lookup_from(&jar);
    let (at, rt) = read_session_cookies(&lookup);
    // Most-strict variant wins on iteration.
    assert_eq!(at.as_deref(), Some("host"));
    assert_eq!(rt.as_deref(), Some("rt"));
    // Bare-only jar: the reader still finds it (setter ran over HTTP).
    let mut bare = HashMap::new();
    bare.insert("hermes_session_at".to_string(), "plain".to_string());
    let (at, rt) = read_session_cookies(&lookup_from(&bare));
    assert_eq!(at.as_deref(), Some("plain"));
    assert_eq!(rt, None);
}

#[test]
fn provider_hint_round_trips() {
    let mut out = Vec::new();
    set_session_provider_cookie(&mut out, "nous", false, "");
    assert_eq!(out.len(), 1);
    // Empty provider: not written.
    let mut out2 = Vec::new();
    set_session_provider_cookie(&mut out2, "", false, "");
    assert!(out2.is_empty());

    let mut jar = HashMap::new();
    jar.insert("hermes_session_provider".to_string(), "nous".to_string());
    assert_eq!(
        read_session_provider(&lookup_from(&jar)).as_deref(),
        Some("nous")
    );
}

#[test]
fn pkce_and_sso_attempt_cookies() {
    let mut out = Vec::new();
    set_pkce_cookie(&mut out, "pkce-payload", true, "");
    assert_eq!(out[0].name, "__Host-hermes_session_pkce");
    assert_eq!(out[0].max_age, PKCE_MAX_AGE);
    clear_pkce_cookie(&mut out, "");
    let deletions = &out[1..];
    assert_eq!(deletions.len(), NAME_VARIANTS.len());
    assert!(deletions.iter().all(|c| c.max_age == 0));

    let mut sso = Vec::new();
    set_sso_attempt_cookie(&mut sso, false, "");
    assert_eq!(sso[0].name, "hermes_sso_attempt");
    assert_eq!(sso[0].value, "1", "constant marker; only presence matters");
    assert_eq!(sso[0].max_age, SSO_ATTEMPT_MAX_AGE);
    clear_sso_attempt_cookie(&mut sso, "");
    assert_eq!(sso[1..].len(), NAME_VARIANTS.len());
}

#[test]
fn https_detection_and_name_variants() {
    assert!(detect_https(Some("https")));
    assert!(!detect_https(Some("http")));
    assert!(!detect_https(None));
    assert_eq!(NAME_VARIANTS, ["__Host-", "__Secure-", ""]);
}
