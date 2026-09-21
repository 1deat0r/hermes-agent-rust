//! Parity tests for `hermes_cli/dashboard_auth/{base,registry,__init__}.py`
//! @ 5d59366. Registry cases mirror the registry module's contract
//! (global + scoped overlays, snapshot/restore, global upsert); base
//! dataclass/ABC cases mirror
//! `tests/hermes_cli/test_dashboard_auth_provider_base.py`.

use std::sync::Arc;

use async_trait::async_trait;

use hermes_cli::dashboard_auth::base::{
    assert_protocol_compliance, DashboardAuthProvider, LoginStart, ProviderError, Session,
    TokenPrincipal,
};
use hermes_cli::dashboard_auth::registry::{
    clear_providers, get_provider, list_providers, list_session_providers, list_token_providers,
    register_global_provider, register_provider, restore_registration, snapshot_registration,
    unregister_global_provider,
};

/// Minimal OAuth session provider.
struct OAuthProvider;

#[async_trait]
impl DashboardAuthProvider for OAuthProvider {
    fn name(&self) -> &str {
        "nous"
    }
    fn display_name(&self) -> &str {
        "Nous Portal"
    }
    async fn start_login(&self, redirect_uri: &str) -> Result<LoginStart, ProviderError> {
        Ok(LoginStart {
            redirect_url: format!("https://portal/oauth/authorize?redirect={redirect_uri}"),
            cookie_payload: vec![("pkce".to_string(), "state123".to_string())],
        })
    }
    async fn complete_login(
        &self,
        code: &str,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Session {
            user_id: "u1".to_string(),
            email: "u1@example.com".to_string(),
            display_name: "User One".to_string(),
            org_id: String::new(),
            provider: "nous".to_string(),
            expires_at: 1_800_000_000,
            access_token: code.to_string(),
            refresh_token: "r1".to_string(),
        })
    }
    async fn verify_session(&self, access_token: &str) -> Result<Option<Session>, ProviderError> {
        if access_token.is_empty() {
            return Ok(None);
        }
        Ok(Some(Session {
            user_id: "u1".to_string(),
            email: String::new(),
            display_name: String::new(),
            org_id: String::new(),
            provider: "nous".to_string(),
            expires_at: 1_800_000_000,
            access_token: access_token.to_string(),
            refresh_token: String::new(),
        }))
    }
    async fn refresh_session(
        &self,
        _refresh_token: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        Err(Box::new(
            hermes_cli::dashboard_auth::base::RefreshExpiredError("dead token".to_string()),
        ))
    }
    async fn revoke_session(&self, _refresh_token: &str) {}
}

/// Token-only service-account provider.
struct TokenProvider;

#[async_trait]
impl DashboardAuthProvider for TokenProvider {
    fn name(&self) -> &str {
        "drain"
    }
    fn display_name(&self) -> &str {
        "Drain Bearer"
    }
    fn supports_session(&self) -> bool {
        false
    }
    fn supports_token(&self) -> bool {
        true
    }
    async fn start_login(&self, _redirect_uri: &str) -> Result<LoginStart, ProviderError> {
        Err(ProviderError("token-only provider".to_string()))
    }
    async fn complete_login(
        &self,
        _code: &str,
        _state: &str,
        _verifier: &str,
        _redirect_uri: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        Err(Box::new(ProviderError("token-only provider".to_string())))
    }
    async fn verify_session(&self, _access_token: &str) -> Result<Option<Session>, ProviderError> {
        Ok(None)
    }
    async fn refresh_session(
        &self,
        _refresh_token: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        Err(Box::new(ProviderError("token-only provider".to_string())))
    }
    async fn revoke_session(&self, _refresh_token: &str) {}

    async fn verify_token(&self, token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        if token == "good-token" {
            Ok(Some(TokenPrincipal {
                principal: "svc:drain".to_string(),
                provider: "drain".to_string(),
                scopes: vec!["read".to_string()],
            }))
        } else {
            // Unrecognised tokens are Ok(None) — never an error — so the
            // seam falls through to the next provider.
            Ok(None)
        }
    }
}

#[test]
fn protocol_compliance_rejects_missing_names() {
    struct NoName;
    #[async_trait]
    impl DashboardAuthProvider for NoName {
        fn name(&self) -> &str {
            ""
        }
        fn display_name(&self) -> &str {
            ""
        }
        async fn start_login(&self, _redirect_uri: &str) -> Result<LoginStart, ProviderError> {
            unreachable!()
        }
        async fn complete_login(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
            unreachable!()
        }
        async fn verify_session(&self, _: &str) -> Result<Option<Session>, ProviderError> {
            unreachable!()
        }
        async fn refresh_session(
            &self,
            _: &str,
        ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
            unreachable!()
        }
        async fn revoke_session(&self, _: &str) {}
    }
    let err = assert_protocol_compliance(&NoName).unwrap_err();
    assert!(
        err.contains("missing or empty attribute: \"name\""),
        "{err}"
    );
    assert!(assert_protocol_compliance(&OAuthProvider).is_ok());
}

#[test]
fn registry_register_get_and_order() {
    clear_providers();
    register_provider(Arc::new(OAuthProvider), None).unwrap();
    let token: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_provider(Arc::clone(&token), None).unwrap();

    // Duplicate registration raises ValueError upstream.
    assert!(register_provider(Arc::new(OAuthProvider), None)
        .unwrap_err()
        .contains("already registered"));

    assert_eq!(list_providers(None).len(), 2);
    assert_eq!(list_providers(None)[0].name(), "nous", "registration order");
    assert_eq!(get_provider("nous", None).unwrap().name(), "nous");
    assert!(get_provider("nope", None).is_none());

    // Token/session subsets filter on the capability flags.
    assert_eq!(list_token_providers().len(), 1);
    assert_eq!(list_token_providers()[0].name(), "drain");
    assert_eq!(list_session_providers().len(), 1);
    assert_eq!(list_session_providers()[0].name(), "nous");
    clear_providers();
    assert!(list_providers(None).is_empty());
}

/// PARITY: scoped overlays (`_scoped_providers`, `_merged`, `_target`) —
/// a scope sees the global map plus its own entries shadowing by name.
#[test]
fn registry_scoped_overlay_merges_and_shadows() {
    clear_providers();
    register_provider(Arc::new(OAuthProvider), None).unwrap();
    register_provider(Arc::new(TokenProvider), Some("home-a")).unwrap();

    // Scoped view: global + overlay.
    let names: Vec<_> = list_providers(Some("home-a"))
        .iter()
        .map(|p| p.name().to_string())
        .collect();
    assert_eq!(names, vec!["nous".to_string(), "drain".to_string()]);
    // Other scopes and the global view are unaffected.
    assert_eq!(list_providers(Some("home-b")).len(), 1);
    assert_eq!(list_providers(None).len(), 1);
    assert_eq!(
        get_provider("drain", Some("home-a")).unwrap().name(),
        "drain"
    );
    assert!(get_provider("drain", None).is_none());

    // A scoped same-name registration shadows the global entry and
    // counts as a duplicate in the effective view.
    register_provider(Arc::new(TokenProvider), None).unwrap();
    assert!(register_provider(Arc::new(TokenProvider), Some("home-a"))
        .unwrap_err()
        .contains("already registered"));
    clear_providers();
}

/// PARITY: `snapshot_registration` / `restore_registration` — the plugin
/// manager's identity-conditional teardown seam.
#[test]
fn registry_snapshot_restore_is_identity_conditional() {
    clear_providers();
    let first: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_provider(Arc::clone(&first), None).unwrap();

    // Snapshot reads the slot without merging.
    assert!(snapshot_registration("drain", None).is_some());
    assert!(snapshot_registration("drain", Some("home-a")).is_none());

    // Restoring while a *different* object is current is a no-op.
    let second: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_global_provider(Arc::clone(&second)).unwrap();
    assert!(!restore_registration("drain", &first, None, None));
    assert_eq!(get_provider("drain", None).unwrap().name(), "drain");

    // Restoring the current object with no previous removes it.
    assert!(restore_registration("drain", &second, None, None));
    assert!(get_provider("drain", None).is_none());
    clear_providers();
}

/// PARITY: `register_global_provider` upserts in place and
/// `unregister_global_provider` only clears the still-current object.
#[test]
fn registry_global_upsert_and_targeted_unload() {
    clear_providers();
    let first: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_global_provider(Arc::clone(&first)).unwrap();
    // Re-discovery rotates in place instead of raising.
    let second: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_global_provider(Arc::clone(&second)).unwrap();
    assert_eq!(list_providers(None).len(), 1);

    // A stale handle never clears the live provider (#91701).
    assert!(!unregister_global_provider("drain", &first));
    assert!(get_provider("drain", None).is_some());
    assert!(unregister_global_provider("drain", &second));
    assert!(get_provider("drain", None).is_none());
    clear_providers();
}

#[test]
fn token_provider_verifies_and_rejects() {
    let provider = TokenProvider;
    // Note: the default `verify_token` raises loudly when supports_token is
    // set without an override; this override recognises exactly one token.
    let principal = futures::executor::block_on(provider.verify_token("good-token"));
    assert_eq!(principal.unwrap().unwrap().principal, "svc:drain");
    let unknown = futures::executor::block_on(provider.verify_token("other"));
    assert_eq!(unknown.unwrap(), None, "unrecognised -> Ok(None)");
}

#[test]
fn oauth_only_provider_defaults_fail_loudly() {
    let provider = OAuthProvider;
    // `complete_password_login` default raises NotImplementedError upstream
    // (fail loudly, never silently accept credentials).
    let result = futures::executor::block_on(provider.complete_password_login("u", "p"));
    let err = result.err().expect("default errors");
    assert!(err.to_string().contains("does not support password login"));
    let result = futures::executor::block_on(provider.verify_token("anything"));
    assert!(result.is_err(), "supports_token defaults to false -> loud");
}

/// PARITY: `test_session_has_required_fields`
/// (`tests/hermes_cli/test_dashboard_auth_provider_base.py`) — every
/// dataclass field is constructible and readable.
#[test]
fn session_dataclass_has_required_fields() {
    let session = Session {
        user_id: "u1".to_string(),
        email: "a@b.com".to_string(),
        display_name: "A".to_string(),
        org_id: "org_1".to_string(),
        provider: "test".to_string(),
        expires_at: 1234567890,
        access_token: "at".to_string(),
        refresh_token: "rt".to_string(),
    };
    assert_eq!(session.user_id, "u1");
    assert_eq!(session.provider, "test");
    assert_eq!(session.expires_at, 1234567890);
    // TokenPrincipal defaults to unscoped; LoginStart carries the
    // PKCE/CSRF cookie payload as an ordered pair list.
    let principal = TokenPrincipal {
        principal: "svc".to_string(),
        provider: "drain".to_string(),
        scopes: vec![],
    };
    assert!(principal.scopes.is_empty(), "unscoped by default");
    let start = LoginStart {
        redirect_url: "https://idp/authorize".to_string(),
        cookie_payload: vec![("pkce".to_string(), "state123".to_string())],
    };
    assert_eq!(start.cookie_payload[0].0, "pkce");
}

#[test]
fn session_round_trip_through_start_and_complete_login() {
    futures::executor::block_on(async {
        let start = OAuthProvider
            .start_login("http://localhost:8080/auth/callback")
            .await
            .unwrap();
        assert!(start.redirect_url.contains("/oauth/authorize"));
        assert_eq!(start.cookie_payload[0].0, "pkce");
        let session = OAuthProvider
            .complete_login(
                "code-1",
                "state",
                "verifier",
                "http://localhost:8080/auth/callback",
            )
            .await
            .unwrap();
        assert_eq!(session.provider, "nous");
        // Empty org_id is the documented convention for providers without
        // orgs.
        assert_eq!(session.org_id, "");
        // verify_session on an empty token is expiry/unknown.
        assert!(OAuthProvider.verify_session("").await.unwrap().is_none());
    })
}
