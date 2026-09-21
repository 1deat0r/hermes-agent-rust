//! Parity tests for `hermes_cli/dashboard_auth/cookies.py` @ 5d59366.
//! Mirrors `tests/hermes_cli/test_dashboard_auth_cookies.py` for the
//! transport-neutral layer (attribute shapes, PKCE codec + compat
//! ladder, deletion shapes); the TestClient HTTP cases belong to the
//! web-server surface.

use std::collections::HashMap;

use hermes_cli::dashboard_auth::cookies::{
    clear_pkce_cookie, clear_session_cookies, clear_sso_attempt_cookie, cookie_path, detect_https,
    encode_pkce_payload, parse_pkce_payload, read_pkce_cookie, read_session_cookies,
    read_session_provider, resolved_name, set_pkce_cookie, set_session_cookies,
    set_session_provider_cookie, set_sso_attempt_cookie, NAME_VARIANTS, PKCE_MAX_AGE, RT_MAX_AGE,
    SSO_ATTEMPT_MAX_AGE,
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
fn clear_session_cookies_prefixed_deletions_carry_secure() {
    // PARITY: `test_clear_session_cookies_prefixed_deletions_carry_secure`
    // — browsers reject a prefixed Set-Cookie that violates its prefix
    // rules, so an insecure __Host- deletion is silently ignored and the
    // session survives logout on HTTPS origins.
    let mut out = Vec::new();
    clear_session_cookies(&mut out, "");
    // 3 cookies x 3 variants.
    assert_eq!(out.len(), 9);
    for bare in [
        "hermes_session_at",
        "hermes_session_rt",
        "hermes_session_provider",
    ] {
        let host = out
            .iter()
            .find(|c| c.name == format!("__Host-{bare}"))
            .expect("host variant");
        assert!(host.secure, "prefixed deletions always carry Secure");
        assert_eq!(host.path, "/", "__Host- requires Path=/");
        assert_eq!(host.max_age, 0);
        let secure = out
            .iter()
            .find(|c| c.name == format!("__Secure-{bare}"))
            .expect("secure variant");
        assert!(secure.secure);
        let bare_del = out.iter().find(|c| c.name == bare).expect("bare");
        // Bare deletion mirrors the bare setter (Lax, no Secure) so it
        // still works on plain-HTTP origins.
        assert!(!bare_del.secure);
        assert_eq!(bare_del.samesite, "lax");
        assert_eq!(bare_del.max_age, 0);
    }
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
fn pkce_cookie_https_is_samesite_none_secure() {
    // PARITY: `test_pkce_cookie_https_is_samesite_none_secure` — the
    // PKCE cookie is set on the /auth/login 302 and must survive the
    // cross-site IDP chain (crbug 40508226).
    let mut payload = HashMap::new();
    payload.insert("provider".to_string(), "stub".to_string());
    payload.insert("state".to_string(), "s".to_string());
    payload.insert("verifier".to_string(), "v".to_string());
    let mut out = Vec::new();
    set_pkce_cookie(&mut out, &payload, true, "");
    assert_eq!(out[0].name, "__Host-hermes_session_pkce");
    assert_eq!(out[0].max_age, PKCE_MAX_AGE);
    assert_eq!(out[0].samesite, "none");
    assert!(out[0].secure);
    assert!(out[0].httponly);
}

#[test]
fn pkce_cookie_http_stays_lax_without_secure() {
    // PARITY: `test_pkce_cookie_http_stays_lax_without_secure` —
    // SameSite=None requires Secure, which HTTP cannot carry.
    let mut payload = HashMap::new();
    payload.insert("provider".to_string(), "stub".to_string());
    let mut out = Vec::new();
    set_pkce_cookie(&mut out, &payload, false, "");
    assert_eq!(out[0].name, "hermes_session_pkce");
    assert_eq!(out[0].samesite, "lax");
    assert!(!out[0].secure);
}

#[test]
fn clear_pkce_cookie_matches_set_shape() {
    // PARITY: `test_clear_pkce_cookie_https_matches_set_shape` +
    // `test_clear_pkce_cookie_http_bare_deletion_is_insecure_lax`.
    let mut out = Vec::new();
    clear_pkce_cookie(&mut out, true, "");
    assert_eq!(out.len(), NAME_VARIANTS.len());
    for deletion in &out {
        assert_eq!(deletion.max_age, 0);
        if deletion.name.starts_with("__") {
            // Prefixed variants require Secure to be valid at all.
            assert!(deletion.secure);
        }
    }
    let prefixed: Vec<_> = out.iter().filter(|c| c.name.starts_with("__")).collect();
    assert!(prefixed.iter().all(|c| c.samesite == "none"));

    let mut out = Vec::new();
    clear_pkce_cookie(&mut out, false, "");
    let bare = out
        .iter()
        .find(|c| c.name == "hermes_session_pkce")
        .expect("bare");
    assert_eq!(bare.samesite, "lax");
    assert!(!bare.secure, "a Secure deletion is ignored on HTTP origins");
}

#[test]
fn pkce_wire_value_is_cookie_octet_base64url_json() {
    // PARITY: `test_set_pkce_cookie_wire_value_is_cookie_octet_base64url_json`
    // — strict proxies (Go net/http) drop the quoted form, so the value
    // must be pure urlsafe base64 (no `;`, quotes, backslashes, `=`).
    let mut payload = HashMap::new();
    payload.insert("provider".to_string(), "stub".to_string());
    payload.insert("state".to_string(), "s".to_string());
    payload.insert("verifier".to_string(), "v".to_string());
    let wire = encode_pkce_payload(&payload);
    assert!(!wire.contains(';') && !wire.contains('"') && !wire.contains('\\'));
    assert!(!wire.contains('='));
    let b64url: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        .chars()
        .collect();
    assert!(wire.chars().all(|c| b64url.contains(&c)));
    assert_eq!(parse_pkce_payload(&wire), payload);
}

#[test]
fn pkce_codec_round_trips_hostile_values() {
    // PARITY: `test_encode_parse_pkce_payload_round_trips_hostile_values`
    // — every char that broke the two previous formats.
    let mut payload = HashMap::new();
    payload.insert("provider".to_string(), "stub".to_string());
    payload.insert("state".to_string(), "s;t=\"a\\te\"".to_string());
    payload.insert("verifier".to_string(), "v=1%3B;x".to_string());
    payload.insert(
        "next".to_string(),
        "/sessions?x=a;b&project=foo%25".to_string(),
    );
    assert_eq!(parse_pkce_payload(&encode_pkce_payload(&payload)), payload);
}

#[test]
fn pkce_parse_old_flat_format_survives_upgrade() {
    // PARITY: `test_parse_pkce_payload_old_format_cookie_survives_rolling_upgrade`
    // — rung 2: split as-is, never unquote first (a `%3B` inside `next`
    // would become a bogus delimiter).
    let old = "provider=stub;state=s123;verifier=v456;\
               next=%2Fsessions%3Fx%3Da%3Bb%26project%3Dfoo";
    let parts = parse_pkce_payload(old);
    assert_eq!(parts["provider"], "stub");
    assert_eq!(parts["state"], "s123");
    assert_eq!(parts["verifier"], "v456");
    assert_eq!(
        parts["next"], "%2Fsessions%3Fx%3Da%3Bb%26project%3Dfoo",
        "still single-encoded, verbatim"
    );
}

#[test]
fn pkce_parse_url_encoded_format_survives_upgrade() {
    // PARITY: `test_parse_pkce_payload_99176_url_encoded_format_survives_upgrade`
    // — rung 3: no raw `;` possible, unquote once then split.
    let wire = "provider%3Dstub%3Bstate%3Ds123%3Bverifier%3Dv456%3Bnext%3D%252Fsessions";
    assert!(!wire.contains(';'));
    let parts = parse_pkce_payload(wire);
    assert_eq!(parts["provider"], "stub");
    assert_eq!(parts["state"], "s123");
    assert_eq!(parts["verifier"], "v456");
    assert_eq!(parts["next"], "%2Fsessions");
}

#[test]
fn pkce_and_sso_attempt_cookies() {
    let mut out = Vec::new();
    let mut payload = HashMap::new();
    payload.insert("provider".to_string(), "stub".to_string());
    set_pkce_cookie(&mut out, &payload, true, "");
    assert_eq!(out[0].name, "__Host-hermes_session_pkce");
    assert_eq!(out[0].max_age, PKCE_MAX_AGE);
    clear_pkce_cookie(&mut out, true, "");
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
