//! Parity tests for `hermes_cli/dashboard_auth/{audit,token_auth}.py`
//! (partial port: middleware Request plumbing PENDING) @ 5d59366.
//! The audit half mirrors `tests/hermes_cli/test_dashboard_auth_audit.py`
//! case-for-case (the b9aa928 missing-test gap is closed upstream); the
//! token_auth half remains source-derived (no dedicated upstream file).
//! Env tests serialize behind a mutex per the workspace convention.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;

use hermes_cli::dashboard_auth::audit::{audit_log, resolve_log_path, AuditEvent};
use hermes_cli::dashboard_auth::base::{DashboardAuthProvider, ProviderError, TokenPrincipal};
use hermes_cli::dashboard_auth::registry::{clear_providers, register_provider};
use hermes_cli::dashboard_auth::token_auth::{
    audit_token_failure, authenticate_token, clear_token_routes, client_ip, extract_bearer_token,
    is_token_route, register_token_route,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// The provider registry is process-global; registry-touching tests are
// serialized (workspace convention for global state).
static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

// ── audit ────────────────────────────────────────────────────────────────

#[test]
fn audit_event_values_are_the_literal_json_fields() {
    assert_eq!(AuditEvent::LoginStart.as_str(), "login_start");
    assert_eq!(AuditEvent::TokenAuthFailure.as_str(), "token_auth_failure");
    assert_eq!(
        AuditEvent::NativeAuthorizeStart.as_str(),
        "native_authorize_start"
    );
    assert_eq!(AuditEvent::WsTicketRejected.as_str(), "ws_ticket_rejected");
}

#[test]
fn audit_log_appends_compact_json_with_redacted_fields_stripped() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };

    audit_log(
        AuditEvent::LoginSuccess,
        &[
            ("provider", json!("nous")),
            // Token-like kwargs are silently dropped before serialisation.
            ("access_token", json!("SUPER-SECRET")),
            ("refresh_token", json!("SUPER-SECRET-2")),
            ("state", json!("csrf-state")),
            ("email", json!("user@example.com")),
        ],
    );
    audit_log(AuditEvent::Logout, &[("provider", json!("nous"))]);

    let log = std::fs::read_to_string(td.path().join("logs/dashboard-auth.log")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["event"], "login_success");
    assert_eq!(first["provider"], "nous");
    assert_eq!(first["email"], "user@example.com");
    assert!(
        first.get("access_token").is_none(),
        "raw token never on disk"
    );
    assert!(first.get("refresh_token").is_none());
    assert!(first.get("state").is_none());
    // Compact separators + ts first-ish shape: a UTC ISO timestamp.
    let ts = first["ts"].as_str().unwrap();
    assert!(ts.ends_with("+00:00") && ts.contains('T'), "{ts}");

    unsafe { std::env::remove_var("HERMES_HOME") };
}

/// ORACLE: `test_audit_writes_jsonlines`
/// (`tests/hermes_cli/test_dashboard_auth_audit.py` @ 5d59366) — two
/// events land as two JSON lines under `$HERMES_HOME/logs/dashboard-auth.log`
/// with the exact field projection (event/provider/user_id/email/ip/ts).
#[test]
fn audit_writes_jsonlines_profile_home_shape() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };

    audit_log(
        AuditEvent::LoginStart,
        &[("provider", json!("nous")), ("ip", json!("1.2.3.4"))],
    );
    audit_log(
        AuditEvent::LoginSuccess,
        &[
            ("provider", json!("nous")),
            ("user_id", json!("u1")),
            ("email", json!("a@b.com")),
            ("ip", json!("1.2.3.4")),
        ],
    );

    let path = td.path().join("logs/dashboard-auth.log");
    assert!(path.exists(), "audit log not created at {}", path.display());
    let raw_log = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = raw_log.lines().collect();
    assert_eq!(lines.len(), 2);
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(second["event"], "login_success");
    assert_eq!(second["provider"], "nous");
    assert_eq!(second["user_id"], "u1");
    assert_eq!(second["email"], "a@b.com");
    assert!(second["ts"].as_str().is_some(), "ISO-8601 timestamp");

    unsafe { std::env::remove_var("HERMES_HOME") };
}

/// ORACLE: `test_audit_redacts_token_like_fields`
/// (`tests/hermes_cli/test_dashboard_auth_audit.py` @ 5d59366) — none of
/// the four token-like values may appear raw anywhere in the log file.
#[test]
fn audit_redacts_token_like_fields_oracle() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };

    audit_log(
        AuditEvent::LoginSuccess,
        &[
            ("provider", json!("nous")),
            ("access_token", json!("should-not-appear")),
            ("refresh_token", json!("also-not")),
            ("code", json!("not-this")),
            ("state", json!("nope")),
        ],
    );
    let raw = std::fs::read_to_string(td.path().join("logs/dashboard-auth.log")).unwrap();
    for forbidden in ["should-not-appear", "also-not", "not-this", "nope"] {
        assert!(
            !raw.contains(forbidden),
            "token-like value leaked: {forbidden}"
        );
    }

    unsafe { std::env::remove_var("HERMES_HOME") };
}

#[test]
fn audit_write_failure_never_raises() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    // A path where logs/ cannot be created (a file blocks the directory).
    std::fs::write(td.path().join("logs"), "not a dir").unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };
    // Must not panic — auth must not fail because the audit logger broke.
    audit_log(AuditEvent::LoginFailure, &[("reason", json!("x"))]);
    unsafe { std::env::remove_var("HERMES_HOME") };
}

// ── token_auth ───────────────────────────────────────────────────────────

#[test]
fn token_route_registry_is_exact_match_and_idempotent() {
    clear_token_routes();
    register_token_route("/api/drain/status");
    register_token_route("/api/drain/status");
    assert!(is_token_route("/api/drain/status"));
    assert!(!is_token_route("/api/drain/status/extra"), "exact match");
    assert!(!is_token_route("/api/other"));
    clear_token_routes();
    assert!(!is_token_route("/api/drain/status"));
}

#[test]
fn bearer_extraction_accepts_bearer_case_insensitively() {
    assert_eq!(extract_bearer_token(Some("Bearer abc123")), "abc123");
    assert_eq!(extract_bearer_token(Some("bearer abc123")), "abc123");
    assert_eq!(extract_bearer_token(Some("BEARER abc123")), "abc123");
    assert_eq!(extract_bearer_token(Some("Bearer   spaced  ")), "spaced");
    // Missing/malformed/non-bearer scheme -> "" (no token presented).
    assert_eq!(extract_bearer_token(Some("")), "");
    assert_eq!(extract_bearer_token(Some("Basic dXNlcjpwYXNz")), "");
    assert_eq!(extract_bearer_token(Some("BearerOnlyNoSpace")), "");
    assert_eq!(extract_bearer_token(None), "");
}

struct AcceptingProvider;
struct RefusingProvider;
struct OutageProvider;

#[async_trait]
impl DashboardAuthProvider for AcceptingProvider {
    fn name(&self) -> &str {
        "accept"
    }
    fn display_name(&self) -> &str {
        "Accept"
    }
    fn supports_token(&self) -> bool {
        true
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
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn verify_session(
        &self,
        _: &str,
    ) -> Result<Option<hermes_cli::dashboard_auth::base::Session>, ProviderError> {
        unreachable!()
    }
    async fn refresh_session(
        &self,
        _: &str,
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn revoke_session(&self, _: &str) {}
    async fn verify_token(&self, token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        if token == "good" {
            Ok(Some(TokenPrincipal {
                principal: "svc".to_string(),
                provider: "accept".to_string(),
                scopes: vec![],
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl DashboardAuthProvider for RefusingProvider {
    fn name(&self) -> &str {
        "refuse"
    }
    fn display_name(&self) -> &str {
        "Refuse"
    }
    fn supports_token(&self) -> bool {
        true
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
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn verify_session(
        &self,
        _: &str,
    ) -> Result<Option<hermes_cli::dashboard_auth::base::Session>, ProviderError> {
        unreachable!()
    }
    async fn refresh_session(
        &self,
        _: &str,
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn revoke_session(&self, _: &str) {}
    async fn verify_token(&self, _token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        Ok(None)
    }
}

struct PanickingProvider;

#[async_trait]
impl DashboardAuthProvider for PanickingProvider {
    fn name(&self) -> &str {
        "panic"
    }
    fn display_name(&self) -> &str {
        "Panic"
    }
    fn supports_token(&self) -> bool {
        true
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
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn verify_session(
        &self,
        _: &str,
    ) -> Result<Option<hermes_cli::dashboard_auth::base::Session>, ProviderError> {
        unreachable!()
    }
    async fn refresh_session(
        &self,
        _: &str,
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn revoke_session(&self, _: &str) {}
    async fn verify_token(&self, _token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        panic!("kaboom")
    }
}

#[async_trait]
impl DashboardAuthProvider for OutageProvider {
    fn name(&self) -> &str {
        "outage"
    }
    fn display_name(&self) -> &str {
        "Outage"
    }
    fn supports_token(&self) -> bool {
        true
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
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn verify_session(
        &self,
        _: &str,
    ) -> Result<Option<hermes_cli::dashboard_auth::base::Session>, ProviderError> {
        unreachable!()
    }
    async fn refresh_session(
        &self,
        _: &str,
    ) -> Result<hermes_cli::dashboard_auth::base::Session, Box<dyn std::error::Error + Send + Sync>>
    {
        unreachable!()
    }
    async fn revoke_session(&self, _: &str) {}
    async fn verify_token(&self, _token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        Err(ProviderError("backing store down".to_string()))
    }
}

fn reset_with(providers: Vec<Arc<dyn DashboardAuthProvider>>) {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_providers();
    for p in providers {
        register_provider(p, None).unwrap();
    }
}

#[test]
fn stacked_providers_accept_recognised_token() {
    clear_providers();
    reset_with(vec![
        Arc::new(RefusingProvider),
        Arc::new(AcceptingProvider),
    ]);
    let (principal, unreachable) = authenticate_token("good", "1.2.3.4", "/api/drain");
    assert_eq!(principal.unwrap().principal, "svc");
    assert_eq!(unreachable, None);
    clear_providers();
}

#[test]
fn no_provider_recognising_yields_401_shape() {
    clear_providers();
    reset_with(vec![Arc::new(RefusingProvider)]);
    let (principal, unreachable) = authenticate_token("unrecognised", "", "/api/drain");
    assert!(principal.is_none());
    assert_eq!(unreachable, None, "plain rejection is 401, not 503");
    clear_providers();
}

#[test]
fn outage_surfaced_only_when_nothing_accepts() {
    clear_providers();
    // Outage + refusing: remember the outage, surface 503.
    reset_with(vec![Arc::new(OutageProvider), Arc::new(RefusingProvider)]);
    let (principal, unreachable) = authenticate_token("t", "", "/api/drain");
    assert!(principal.is_none());
    assert_eq!(unreachable.as_deref(), Some("outage"));

    // Accepting provider AFTER the outage still accepts (the seam keeps
    // trying other providers).
    reset_with(vec![Arc::new(OutageProvider), Arc::new(AcceptingProvider)]);
    let (principal, unreachable) = authenticate_token("good", "", "/api/drain");
    assert_eq!(principal.unwrap().principal, "svc");
    assert_eq!(unreachable, None);
    clear_providers();
}

#[test]
fn empty_token_short_circuits_without_consulting_providers() {
    clear_providers();
    let (principal, unreachable) = authenticate_token("", "", "/api/drain");
    assert!(principal.is_none());
    assert_eq!(unreachable, None);
}

/// PARITY: `test_authenticate_token_buggy_provider_does_not_crash`
/// (`tests/hermes_cli/test_dashboard_token_auth.py`) — a panicking
/// provider is skipped like upstream's `except Exception: continue`,
/// and a later provider still accepts.
#[test]
fn panicking_provider_does_not_crash_the_gate() {
    clear_providers();
    reset_with(vec![
        Arc::new(PanickingProvider),
        Arc::new(AcceptingProvider),
    ]);
    let (principal, unreachable) = authenticate_token("good", "", "/api/drain");
    assert_eq!(principal.unwrap().principal, "svc");
    assert_eq!(unreachable, None);
    clear_providers();
}

#[test]
fn client_ip_prefers_first_forwarded_entry() {
    assert_eq!(
        client_ip(Some("1.1.1.1, 2.2.2.2"), Some("3.3.3.3")),
        "1.1.1.1"
    );
    assert_eq!(client_ip(Some(""), Some("3.3.3.3")), "3.3.3.3");
    assert_eq!(client_ip(None, Some("3.3.3.3")), "3.3.3.3");
    assert_eq!(client_ip(None, None), "");
}

#[test]
fn audit_token_failure_records_both_shapes() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };

    audit_token_failure(Some("outage"), "/api/drain", "1.2.3.4");
    audit_token_failure(None, "/api/drain", "1.2.3.4");

    let log = std::fs::read_to_string(resolve_log_path()).unwrap();
    let lines: Vec<serde_json::Value> = log
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["reason"], "provider_unreachable");
    assert_eq!(lines[0]["provider"], "outage");
    assert_eq!(lines[1]["reason"], "no_provider_recognises_token");
    assert_eq!(lines[1]["path"], "/api/drain");
    unsafe { std::env::remove_var("HERMES_HOME") };
}
