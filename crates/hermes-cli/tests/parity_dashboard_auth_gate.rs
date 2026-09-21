//! Parity tests for the dashboard-auth gate decision tables:
//! `middleware.py` + the pure helpers of `routes.py` + the render
//! decisions of `login_page.py` @ 5d59366. The FastAPI/HTML surfaces
//! own the I/O; every decision ports and is pinned here.

use std::collections::HashMap;

use hermes_cli::dashboard_auth::base::Session;
use hermes_cli::dashboard_auth::login_page::{
    html_escape, login_controls, native_choice_hrefs, next_query_fragment, ProviderControl,
};
use hermes_cli::dashboard_auth::middleware::{
    auto_sso_response, classify_request, next_after_verify, path_is_public, percent_encode_all,
    safe_next_target, unauth_response, GateDecision, SsoDecision, SsoInputs, UnauthResponse,
    GATE_PUBLIC_PREFIXES,
};
use hermes_cli::dashboard_auth::route_logic::{
    bearer_payload, native_finish_url, provider_pkce_segments, redirect_uri,
    reset_password_rate_limit, select_native_provider, validate_loopback_redirect_uri,
    validate_post_login_target, LoopbackError,
};

fn session() -> Session {
    Session {
        user_id: "u1".to_string(),
        email: "u@x".to_string(),
        display_name: "U".to_string(),
        org_id: "o".to_string(),
        provider: "nous".to_string(),
        expires_at: 1_800_000_000,
        access_token: "at".to_string(),
        refresh_token: "rt".to_string(),
    }
}

// ── middleware: public paths ──

#[test]
fn public_bypass_matches_exact_and_prefix() {
    // Exact API paths pass…
    assert!(path_is_public("/api/auth/providers"));
    // …but exact-match means no extension leak.
    assert!(!path_is_public("/api/status/extension"));
    // Bootstrap + assets pass by prefix; lookalikes do not.
    assert!(path_is_public("/auth/login"));
    assert!(path_is_public("/assets/foo.css"));
    assert!(!path_is_public("/assetsleak"));
    assert!(!path_is_public("/api/sessions"));
    assert!(!GATE_PUBLIC_PREFIXES.is_empty());
}

// ── middleware: next target + unauth shapes ──

#[test]
fn safe_next_target_keeps_query_and_encodes() {
    assert_eq!(safe_next_target("/sessions", ""), "%2Fsessions");
    assert_eq!(
        safe_next_target("/sessions", "x=1&y=2"),
        "%2Fsessions%3Fx%3D1%26y%3D2"
    );
    // Dropped deep links fall back to the SPA memory slot.
    assert_eq!(safe_next_target("/api/sessions", ""), "");
    assert_eq!(safe_next_target("/auth/callback", ""), "");
    assert_eq!(safe_next_target("https://evil/", ""), "");
    assert_eq!(safe_next_target("", ""), "");
}

#[test]
fn unauth_api_is_401_envelope_html_is_302() {
    // PARITY: `_unauth_response` — API 401 carries login_url, HTML 302s.
    match unauth_response("/api/sessions", "", "", "no_cookie") {
        UnauthResponse::Api {
            error, login_url, ..
        } => {
            assert_eq!(error, "unauthenticated");
            assert_eq!(login_url, "/login");
        }
        UnauthResponse::Redirect { .. } => panic!("API must not redirect"),
    }
    match unauth_response(
        "/api/sessions",
        "/hermes",
        "/sessions",
        "invalid_or_expired_session",
    ) {
        UnauthResponse::Api {
            error, login_url, ..
        } => {
            assert_eq!(error, "session_expired");
            // The API path itself is never a valid `next` target, so the
            // envelope points at the bare prefixed login.
            assert_eq!(login_url, "/hermes/login");
        }
        UnauthResponse::Redirect { .. } => panic!("API must not redirect"),
    }
    match unauth_response("/sessions", "", "", "no_cookie") {
        UnauthResponse::Redirect { location } => {
            assert_eq!(location, "/login?next=%2Fsessions")
        }
        UnauthResponse::Api { .. } => panic!("HTML must redirect"),
    }
}

#[test]
fn percent_encoding_matches_quote_safe_empty() {
    assert_eq!(percent_encode_all("/a b?c=d&e"), "%2Fa%20b%3Fc%3Dd%26e");
    assert_eq!(percent_encode_all("abc-_.~123"), "abc-_.~123");
}

// ── middleware: auto-SSO ──

fn sso_inputs<'a>(
    path: &'a str,
    marker: bool,
    providers: &'a [String],
    is_password: bool,
    name: &'a str,
) -> SsoInputs<'a> {
    SsoInputs {
        path,
        sso_attempt_present: marker,
        session_providers: providers,
        single_is_password: is_password,
        single_name: name,
    }
}

#[test]
fn auto_sso_redirects_single_oauth_provider() {
    let providers = vec!["nous".to_string()];
    match auto_sso_response(
        &sso_inputs("/sessions", false, &providers, false, "nous"),
        "",
        "",
    ) {
        SsoDecision::Redirect { location } => {
            assert_eq!(location, "/auth/login?provider=nous&next=%2Fsessions");
        }
        _ => panic!("single OAuth provider must silent-bounce"),
    }
}

#[test]
fn auto_sso_marker_clears_and_falls_back() {
    let providers = vec!["nous".to_string()];
    match auto_sso_response(
        &sso_inputs("/sessions", true, &providers, false, "nous"),
        "",
        "",
    ) {
        SsoDecision::ClearAndFallback { .. } => {}
        _ => panic!("present marker must clear, not ping-pong"),
    }
}

#[test]
fn auto_sso_abstains_for_api_or_password_or_many() {
    let one = vec!["nous".to_string()];
    let two = vec!["a".to_string(), "b".to_string()];
    assert_eq!(
        auto_sso_response(
            &sso_inputs("/api/sessions", false, &one, false, "nous"),
            "",
            ""
        ),
        SsoDecision::None
    );
    assert_eq!(
        auto_sso_response(&sso_inputs("/sessions", false, &one, true, "basic"), "", ""),
        SsoDecision::None,
        "password providers must render the form"
    );
    assert_eq!(
        auto_sso_response(&sso_inputs("/sessions", false, &two, false, "a"), "", ""),
        SsoDecision::None,
        "several providers need a chooser"
    );
    assert_eq!(
        auto_sso_response(&sso_inputs("/sessions", false, &[], false, ""), "", ""),
        SsoDecision::None
    );
}

// ── middleware: gate flow ──

#[test]
fn gate_classification_follows_the_branch_order() {
    assert_eq!(
        classify_request(false, false, "/x", None, None, None),
        GateDecision::PassThrough
    );
    assert_eq!(
        classify_request(true, true, "/api/drain", None, None, None),
        GateDecision::AlreadyCleared
    );
    assert_eq!(
        classify_request(true, false, "/api/status", None, None, None),
        GateDecision::AlreadyCleared,
        "public path"
    );
    assert_eq!(
        classify_request(true, false, "/sessions", Some("tok"), None, None),
        GateDecision::VerifyBearer
    );
    assert_eq!(
        classify_request(true, false, "/sessions", None, None, None),
        GateDecision::AttemptSso
    );
    assert_eq!(
        classify_request(true, false, "/sessions", None, Some("at"), Some("rt")),
        GateDecision::VerifySession {
            provider_hint: None
        }
    );
    // Absent AT + present RT skips straight to refresh (the COMMON case).
    assert_eq!(
        classify_request(true, false, "/sessions", None, None, Some("rt")),
        GateDecision::AttemptRefresh
    );
}

#[test]
fn gate_next_step_serves_refreshes_or_expires() {
    assert_eq!(
        next_after_verify(Some(true), false, "nous", true, None),
        GateDecision::ServeWithSession {
            stamp_provider_hint: true
        }
    );
    assert_eq!(
        next_after_verify(Some(true), true, "nous", true, None),
        GateDecision::ServeWithSession {
            stamp_provider_hint: false
        }
    );
    assert_eq!(
        next_after_verify(Some(false), false, "", true, Some(true)),
        GateDecision::ServeWithSession {
            stamp_provider_hint: false
        }
    );
    assert_eq!(
        next_after_verify(Some(false), false, "", true, Some(false)),
        GateDecision::SessionExpired
    );
    assert_eq!(
        next_after_verify(Some(false), false, "", false, None),
        GateDecision::SessionExpired,
        "no RT: nothing to rotate"
    );
    assert_eq!(
        next_after_verify(Some(false), false, "", true, None),
        GateDecision::Unreachable,
        "uncertain, not rejected: cookies kept"
    );
    assert_eq!(
        next_after_verify(None, false, "", true, None),
        GateDecision::Unreachable
    );
}

// ── routes: pure helpers ──

#[test]
fn post_login_target_revalidates_every_hop() {
    assert_eq!(validate_post_login_target("/sessions"), "/sessions");
    assert_eq!(validate_post_login_target("/api/sessions"), "");
    assert_eq!(validate_post_login_target("/auth/callback"), "");
    assert_eq!(validate_post_login_target("https://evil/"), "");
    assert_eq!(validate_post_login_target(""), "");
}

#[test]
fn loopback_redirect_uri_is_a_security_boundary() {
    // PARITY: `_validate_loopback_redirect_uri` — http loopback literals
    // only; `localhost` rejected per RFC 8252 §8.3.
    assert!(validate_loopback_redirect_uri("http://127.0.0.1:8765/cb").is_ok());
    assert!(validate_loopback_redirect_uri("http://[::1]:8765/cb").is_ok());
    assert_eq!(
        validate_loopback_redirect_uri("").unwrap_err(),
        LoopbackError::Missing
    );
    assert_eq!(
        validate_loopback_redirect_uri("https://127.0.0.1/cb").unwrap_err(),
        LoopbackError::NotHttp
    );
    assert_eq!(
        validate_loopback_redirect_uri("http://localhost:8765/cb").unwrap_err(),
        LoopbackError::NotLoopback
    );
    assert_eq!(
        validate_loopback_redirect_uri("http://evil.example.com/cb").unwrap_err(),
        LoopbackError::NotLoopback
    );
}

#[test]
fn native_provider_selection_never_guesses() {
    // PARITY: `_select_native_provider` — explicit wins; empty
    // auto-selects ONLY the single candidate.
    assert_eq!(
        select_native_provider("nous", Some("nous".to_string()), &[]),
        Some("nous".to_string())
    );
    assert_eq!(select_native_provider("ghost", None, &[]), None);
    assert_eq!(
        select_native_provider("", None, &["only".to_string()]),
        Some("only".to_string())
    );
    assert_eq!(
        select_native_provider("", None, &["a".to_string(), "b".to_string()]),
        None,
        "several: the caller renders a chooser"
    );
    assert_eq!(select_native_provider("", None, &[]), None);
}

#[test]
fn bearer_payload_carries_tokens_in_body() {
    let payload = bearer_payload(&session());
    assert_eq!(payload["access_token"], "at");
    assert_eq!(payload["refresh_token"], "rt");
    assert_eq!(payload["token_type"], "Bearer");
    assert_eq!(payload["expires_at"], 1_800_000_000);
    assert_eq!(payload["provider"], "nous");
    assert_eq!(payload["user_id"], "u1");
}

#[test]
fn pkce_segments_parse_the_flat_form_once() {
    let mut payload = HashMap::new();
    payload.insert(
        "hermes_session_pkce".to_string(),
        "state=s;verifier=v".to_string(),
    );
    let segments = provider_pkce_segments(&payload);
    assert_eq!(segments["state"], "s");
    assert_eq!(segments["verifier"], "v");
    assert!(provider_pkce_segments(&HashMap::new()).is_empty());
}

#[test]
fn redirect_uri_prefers_public_url_without_doubling() {
    // Operator URL is the complete authority (prefix ignored).
    assert_eq!(
        redirect_uri("https://h.example.com/hermes", "https://in/cb", "/hermes"),
        "https://h.example.com/hermes/auth/callback"
    );
    // Otherwise the app URL with the prefix prepended to its path.
    assert_eq!(
        redirect_uri("", "https://h.example.com/auth/callback", "/hermes"),
        "https://h.example.com/hermes/auth/callback"
    );
    assert_eq!(
        redirect_uri("", "https://h.example.com/auth/callback", ""),
        "https://h.example.com/auth/callback"
    );
}

#[test]
fn native_finish_url_assembles_loopback_redirect() {
    assert_eq!(
        native_finish_url("http://127.0.0.1:8765/cb", "gw-code", "cs"),
        "http://127.0.0.1:8765/cb?code=gw-code&state=cs"
    );
    assert_eq!(
        native_finish_url("http://127.0.0.1:1/cb?x=1", "c d", "s"),
        "http://127.0.0.1:1/cb?x=1&code=c%20d&state=s"
    );
}

// ── login_page: escaping + shapes ──

#[test]
fn login_controls_escape_everything() {
    // Empty roster → no controls (web surface renders _EMPTY_HTML).
    assert!(login_controls(&[], "/sessions").is_empty());
    let controls = login_controls(
        &[
            ("nous".to_string(), "Nous <Portal>".to_string(), false),
            ("basic".to_string(), "Basic".to_string(), true),
        ],
        "/sessions",
    );
    match &controls[0] {
        ProviderControl::OAuth { href, label } => {
            assert!(href.starts_with("/auth/login?provider=nous&next="));
            assert_eq!(label, "Nous &lt;Portal&gt;");
        }
        _ => panic!("OAuth provider must render an anchor"),
    }
    match &controls[1] {
        ProviderControl::Password { provider } => assert_eq!(provider, "basic"),
        _ => panic!("password provider must render the form marker"),
    }
    // next="" → no next fragment (byte-identical round trip shape).
    let plain = login_controls(&[("n".to_string(), "N".to_string(), false)], "");
    match &plain[0] {
        ProviderControl::OAuth { href, .. } => assert_eq!(href, "/auth/login?provider=n"),
        _ => panic!("anchor expected"),
    }
}

#[test]
fn next_fragment_matches_gate_encoding() {
    assert_eq!(next_query_fragment(""), "");
    assert_eq!(next_query_fragment("/sessions"), "&next=%2Fsessions");
    // Hostile next values cannot break the attribute.
    assert!(!next_query_fragment("/s\" onclick=\"x").contains('"'));
}

#[test]
fn native_choice_links_carry_pkce_inputs() {
    let hrefs = native_choice_hrefs(
        &[("nous".to_string(), "Nous".to_string())],
        "/auth/native/authorize",
        &[
            ("code_challenge".to_string(), "cc".to_string()),
            ("code_challenge_method".to_string(), "S256".to_string()),
            (
                "redirect_uri".to_string(),
                "http://127.0.0.1:1/cb".to_string(),
            ),
            ("state".to_string(), "st".to_string()),
        ],
    );
    assert_eq!(hrefs.len(), 1);
    assert!(hrefs[0].0.contains("/auth/native/authorize?"));
    assert!(hrefs[0].0.contains("provider=nous"));
    assert!(hrefs[0].0.contains("code_challenge=cc"));
}

#[test]
fn html_escape_contexts_match_python() {
    assert_eq!(html_escape("<a>&", false), "&lt;a&gt;&amp;");
    assert_eq!(html_escape("\"'&<>", true), "&quot;&#x27;&amp;&lt;&gt;");
    assert_eq!(next_query_fragment("/s"), "&next=%2Fs");
    let _ = reset_password_rate_limit;
}
