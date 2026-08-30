//! Parity tests for `hermes_cli/dashboard_auth/{base,registry,__init__}.py`
//! @ b9aa928. Upstream has no dedicated test file for these leaves
//! (missing-test gap, noted in the ledger); cases derive from the upstream
//! code as oracle.

use std::sync::Arc;

use async_trait::async_trait;

use hermes_cli::dashboard_auth::base::{
    assert_protocol_compliance, DashboardAuthProvider, LoginStart, ProviderError, Session,
    TokenPrincipal,
};
use hermes_cli::dashboard_auth::registry::{
    clear_providers, get_provider, list_providers, list_session_providers, list_token_providers,
    register_provider,
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
    register_provider(Arc::new(OAuthProvider)).unwrap();
    let token: Arc<dyn DashboardAuthProvider> = Arc::new(TokenProvider);
    register_provider(Arc::clone(&token)).unwrap();

    // Duplicate registration raises ValueError upstream.
    assert!(register_provider(Arc::new(OAuthProvider))
        .unwrap_err()
        .contains("already registered"));

    assert_eq!(list_providers().len(), 2);
    assert_eq!(list_providers()[0].name(), "nous", "registration order");
    assert_eq!(get_provider("nous").unwrap().name(), "nous");
    assert!(get_provider("nope").is_none());

    // Token/session subsets filter on the capability flags.
    assert_eq!(list_token_providers().len(), 1);
    assert_eq!(list_token_providers()[0].name(), "drain");
    assert_eq!(list_session_providers().len(), 1);
    assert_eq!(list_session_providers()[0].name(), "nous");
    clear_providers();
    assert!(list_providers().is_empty());
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
