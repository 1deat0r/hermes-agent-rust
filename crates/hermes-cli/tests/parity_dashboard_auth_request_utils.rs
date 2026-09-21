//! Parity tests for `hermes_cli/dashboard_auth/request_utils.py` @
//! 5d59366 (whole module, 89 lines). The `Request`-taking entry points
//! are exercised through their header values (the FastAPI surface owns
//! the `Request`); `scan_session_providers` runs sync closures over the
//! registry.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use hermes_cli::dashboard_auth::base::{DashboardAuthProvider, LoginStart, ProviderError, Session};
use hermes_cli::dashboard_auth::registry::{clear_providers, register_provider};
use hermes_cli::dashboard_auth::request_utils::{
    access_token_max_age, client_ip, extract_bearer, is_safe_next_path, scan_session_providers,
    unreachable_detail, ScanOutcome, NEXT_DENY_PREFIXES, UNREACHABLE_STATUS,
};

static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

struct StubProvider {
    name: &'static str,
    session: bool,
}

#[async_trait]
impl DashboardAuthProvider for StubProvider {
    fn name(&self) -> &str {
        self.name
    }
    fn display_name(&self) -> &str {
        "Stub"
    }
    fn supports_session(&self) -> bool {
        self.session
    }
    async fn start_login(&self, _: &str) -> Result<LoginStart, ProviderError> {
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

#[test]
fn deny_prefixes_pin_the_redirect_surface() {
    assert_eq!(NEXT_DENY_PREFIXES, &["/login", "/auth/", "/api/auth/"]);
}

#[test]
fn client_ip_prefers_first_forwarded_hop() {
    assert_eq!(client_ip("1.1.1.1, 2.2.2.2", "3.3.3.3"), "1.1.1.1");
    assert_eq!(client_ip("", "3.3.3.3"), "3.3.3.3");
    assert_eq!(client_ip("", ""), "");
}

#[test]
fn bearer_extraction_matches_request_utils() {
    assert_eq!(extract_bearer("Bearer abc123"), "abc123");
    assert_eq!(extract_bearer("bearer abc123"), "abc123");
    assert_eq!(extract_bearer("Basic dXNlcjpwYXNz"), "");
    assert_eq!(extract_bearer("BearerOnlyNoSpace"), "");
    assert_eq!(extract_bearer(""), "");
}

/// PARITY: `is_safe_next_path` (+ `test_safe_next_validator_*` in
/// `test_dashboard_auth_401_reauth.py`) — same-origin targets only,
/// never auth routes or `/api`.
#[test]
fn safe_next_path_rejects_open_redirects() {
    assert!(is_safe_next_path("/sessions"));
    assert!(is_safe_next_path("/"));
    assert!(!is_safe_next_path("https://evil.example.com/"));
    assert!(!is_safe_next_path("//evil.example.com/"));
    assert!(!is_safe_next_path("/login"));
    assert!(!is_safe_next_path("/auth/callback"));
    assert!(!is_safe_next_path("/api/auth/ws-ticket"));
    // `/api` prefix lookalikes that are NOT api paths still pass.
    assert!(is_safe_next_path("/apiculture"));
    assert!(!is_safe_next_path("/api"));
    assert!(!is_safe_next_path("/api/sessions"));
}

#[test]
fn access_token_max_age_floors_at_sixty() {
    assert_eq!(access_token_max_age(1_000_900, 1_000_000), 900);
    assert_eq!(access_token_max_age(1_000_030, 1_000_000), 60);
    assert_eq!(access_token_max_age(999_000, 1_000_000), 60);
}

#[test]
fn unreachable_shape_is_503_with_provider_name() {
    assert_eq!(UNREACHABLE_STATUS, 503);
    assert_eq!(
        unreachable_detail("nous"),
        "Auth provider \"nous\" unreachable"
    );
}

/// PARITY: `scan_session_providers` — hint goes first, first hit wins,
/// unreachable remembers without aborting, all-reject is `None`,
/// unreachable-only is `Err`.
#[test]
fn scan_hint_first_hit_wins() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_providers();
    register_provider(
        Arc::new(StubProvider {
            name: "a",
            session: true,
        }),
        None,
    )
    .unwrap();
    register_provider(
        Arc::new(StubProvider {
            name: "b",
            session: true,
        }),
        None,
    )
    .unwrap();
    let seen: Mutex<Vec<String>> = Mutex::new(vec![]);
    let result = scan_session_providers(Some("b"), |p| {
        seen.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(p.name().to_string());
        if p.name() == "b" {
            ScanOutcome::Done(p.name().to_string())
        } else {
            ScanOutcome::Next
        }
    });
    assert_eq!(result.unwrap().as_deref(), Some("b"));
    assert_eq!(
        seen.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        vec!["b".to_string()],
        "hinted provider goes first; scan stops at first hit"
    );
    clear_providers();
}

#[test]
fn scan_unreachable_does_not_abort_the_chain() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_providers();
    register_provider(
        Arc::new(StubProvider {
            name: "down",
            session: true,
        }),
        None,
    )
    .unwrap();
    register_provider(
        Arc::new(StubProvider {
            name: "up",
            session: true,
        }),
        None,
    )
    .unwrap();
    // Unreachable first, hit second: the hit wins, no error.
    let result = scan_session_providers(None, |p| {
        if p.name() == "down" {
            ScanOutcome::Unreachable
        } else {
            ScanOutcome::Done("hit".to_string())
        }
    });
    assert_eq!(result.unwrap().as_deref(), Some("hit"));
    // All unreachable: Err carries the FIRST unreachable name.
    let result: Result<Option<String>, ProviderError> =
        scan_session_providers(None, |_| ScanOutcome::Unreachable);
    assert_eq!(result.unwrap_err().to_string(), "provider error: down");
    // All reject: None (caller forces re-login, not 503).
    let result: Result<Option<String>, ProviderError> =
        scan_session_providers(None, |_| ScanOutcome::Next);
    assert!(result.unwrap().is_none());
    // Stale hint leaves registration order intact.
    let mut order = vec![];
    let _: Result<Option<String>, ProviderError> = scan_session_providers(Some("ghost"), |p| {
        order.push(p.name().to_string());
        ScanOutcome::Next
    });
    assert_eq!(order, vec!["down".to_string(), "up".to_string()]);
    clear_providers();
}
