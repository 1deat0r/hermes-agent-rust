//! Parity tests for
//! `hermes_cli/dashboard_auth/refresh_singleflight.py` @ 5d59366 (whole
//! module, 107 lines). The module docstring's contract — one flight per
//! rotating credential, success/dead caching, unreachable passthrough,
//! hint-ordered discovery — is exercised through sync closures over
//! stub providers.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;

use hermes_cli::dashboard_auth::base::{DashboardAuthProvider, ProviderError, Session};
use hermes_cli::dashboard_auth::refresh_singleflight::{
    refresh_provider_coalesced, refresh_session_coalesced, reset_for_tests, RefreshCallError,
    FAILURE_TTL, MAX_ENTRIES, SUCCESS_TTL,
};
use hermes_cli::dashboard_auth::registry::{clear_providers, register_provider};

static SERIAL: Mutex<()> = Mutex::new(());

struct StubProvider {
    name: &'static str,
}

fn session(provider: &str) -> Session {
    Session {
        user_id: "u".to_string(),
        email: "u@x".to_string(),
        display_name: "U".to_string(),
        org_id: String::new(),
        provider: provider.to_string(),
        expires_at: 1_800_000_000,
        access_token: "at".to_string(),
        refresh_token: "rt".to_string(),
    }
}

#[async_trait]
impl DashboardAuthProvider for StubProvider {
    fn name(&self) -> &str {
        self.name
    }
    fn display_name(&self) -> &str {
        "Stub"
    }
    async fn start_login(
        &self,
        _: &str,
    ) -> Result<hermes_cli::dashboard_auth::base::LoginStart, ProviderError> {
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

fn register(names: &[&'static str]) {
    clear_providers();
    reset_for_tests();
    for name in names {
        register_provider(Arc::new(StubProvider { name }), None).unwrap();
    }
}

fn provider(name: &str) -> Arc<dyn DashboardAuthProvider> {
    hermes_cli::dashboard_auth::registry::get_provider(name, None).unwrap()
}

#[test]
fn constants_pin_the_burst_windows() {
    assert_eq!(SUCCESS_TTL, std::time::Duration::from_secs(30));
    assert_eq!(FAILURE_TTL, std::time::Duration::from_secs(5));
    assert_eq!(MAX_ENTRIES, 256);
}

#[test]
fn burst_coalesces_to_one_provider_call() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    register(&["nous"]);
    let calls = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let calls = Arc::clone(&calls);
            let p = provider("nous");
            std::thread::spawn(move || {
                refresh_provider_coalesced(&p, "rt-burst", || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    // Hold the flight so the burst actually overlaps.
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    Ok(session("nous"))
                })
                .expect("refresh succeeds")
                .expect("session")
            })
        })
        .collect();
    let mut providers = std::collections::HashSet::new();
    for h in handles {
        providers.insert(h.join().expect("no deadlock").provider);
    }
    assert_eq!(providers, ["nous".to_string()].into_iter().collect());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "exactly one caller reaches the provider"
    );
}

#[test]
fn dead_token_caches_briefly_and_rejects() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    register(&["nous"]);
    let calls = AtomicUsize::new(0);
    let p = provider("nous");
    for _ in 0..3 {
        let out = refresh_provider_coalesced(&p, "rt-dead", || {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(RefreshCallError::Expired)
        })
        .expect("expired is not an error");
        assert!(out.is_none());
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "dead token served from cache"
    );
}

#[test]
fn unreachable_is_never_cached() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    register(&["nous"]);
    let calls = AtomicUsize::new(0);
    let p = provider("nous");
    for _ in 0..2 {
        let err = refresh_provider_coalesced(&p, "rt-down", || {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(RefreshCallError::Unreachable(ProviderError(
                "down".to_string(),
            )))
        })
        .unwrap_err();
        assert_eq!(err.to_string(), "provider error: down");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2, "every attempt retries");
}

#[test]
fn coalesced_hint_orders_and_returns_name() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    register(&["a", "b"]);
    let mut rejected = vec![];
    let out = refresh_session_coalesced("rt-x", Some("b"), &mut rejected, &|p, _| {
        if p.name() == "b" {
            Ok(session("b"))
        } else {
            Err(RefreshCallError::Expired)
        }
    })
    .expect("scan succeeds");
    // Hinted provider goes first and wins; the other is never consulted.
    assert_eq!(out.unwrap().1, "b");
    assert!(rejected.is_empty());
}

#[test]
fn coalesced_all_reject_is_none_unreachable_is_503() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    register(&["a", "b"]);
    // All reject → None (caller forces re-login, not 503).
    let mut rejected = vec![];
    let out = refresh_session_coalesced("rt-dead", None, &mut rejected, &|_, _| {
        Err(RefreshCallError::Expired)
    })
    .expect("scan completes");
    assert!(out.is_none());
    assert_eq!(rejected.len(), 2);
    // Nothing rotates + one unreachable → 503 propagates.
    let mut rejected = vec![];
    let err = refresh_session_coalesced("rt-down", None, &mut rejected, &|p, _| {
        if p.name() == "a" {
            Err(RefreshCallError::Unreachable(ProviderError(
                "a".to_string(),
            )))
        } else {
            Err(RefreshCallError::Expired)
        }
    })
    .unwrap_err();
    assert_eq!(err.to_string(), "provider error: a");
    clear_providers();
    reset_for_tests();
}
