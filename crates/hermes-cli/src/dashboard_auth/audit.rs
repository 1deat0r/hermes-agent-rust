//! Audit log for dashboard-auth events.
//!
//! PARITY: `hermes_cli/dashboard_auth/audit.py` @ b9aa928 (whole module).
//!
//! Profile-aware location: `$HERMES_HOME/logs/dashboard-auth.log`. One
//! JSON object per line. Token-like fields are stripped before
//! serialisation to avoid leaking refresh tokens or JWTs to disk.
//!
//! Deliberately minimal dependency surface (hermes-constants only) so it
//! can be imported from middleware code that loads early in startup.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::{json, Value};

use hermes_constants::get_hermes_home;

static WRITE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Field names that must never appear in the log raw. Any kwarg matching
/// these is silently dropped.
///
/// PARITY: `_REDACTED_FIELDS` (upstream lines 24-28).
const REDACTED_FIELDS: [&str; 9] = [
    "access_token",
    "refresh_token",
    "code",
    "code_verifier",
    "state",
    "ticket",
    "cookie",
    "Authorization",
    "authorization",
];

/// Event types written to dashboard-auth.log. Values are the literal
/// `event` field on the JSON line.
///
/// PARITY: `AuditEvent` (upstream lines 31-59).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEvent {
    LoginStart,
    LoginSuccess,
    LoginFailure,
    Logout,
    RefreshSuccess,
    RefreshFailure,
    Revoke,
    SessionVerifyFailure,
    WsTicketMinted,
    WsTicketRejected,
    TokenAuthSuccess,
    TokenAuthFailure,
    // RFC 8252 native-app (system-browser + loopback + PKCE) flow.
    NativeAuthorizeStart,
    NativeCodeIssued,
    NativeTokenSuccess,
    NativeTokenFailure,
}

impl AuditEvent {
    /// The literal `event` field value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginStart => "login_start",
            Self::LoginSuccess => "login_success",
            Self::LoginFailure => "login_failure",
            Self::Logout => "logout",
            Self::RefreshSuccess => "refresh_success",
            Self::RefreshFailure => "refresh_failure",
            Self::Revoke => "revoke",
            Self::SessionVerifyFailure => "session_verify_failure",
            Self::WsTicketMinted => "ws_ticket_minted",
            Self::WsTicketRejected => "ws_ticket_rejected",
            Self::TokenAuthSuccess => "token_auth_success",
            Self::TokenAuthFailure => "token_auth_failure",
            Self::NativeAuthorizeStart => "native_authorize_start",
            Self::NativeCodeIssued => "native_code_issued",
            Self::NativeTokenSuccess => "native_token_success",
            Self::NativeTokenFailure => "native_token_failure",
        }
    }
}

/// `$HERMES_HOME/logs/dashboard-auth.log` — via `get_hermes_home()` so
/// profile overrides and the native-Windows `%LOCALAPPDATA%` fallback are
/// honored.
///
/// PARITY: `_resolve_log_path` (upstream lines 62-70).
pub fn resolve_log_path() -> PathBuf {
    get_hermes_home().join("logs").join("dashboard-auth.log")
}

/// Append one event to the audit log.
///
/// Token-like fields are dropped. Missing log directory is created. Write
/// failures are logged at WARNING but never raise — auth must not fail
/// because the audit logger broke.
///
/// PARITY: `audit_log` (upstream lines 73-96). The timestamp is
/// timezone-aware UTC ISO-8601; the line is compact JSON
/// (`separators=(",", ":")`).
pub fn audit_log(event: AuditEvent, fields: &[(&str, Value)]) {
    let mut entry = serde_json::Map::new();
    entry.insert(
        "ts".to_string(),
        json!(Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()),
    );
    entry.insert("event".to_string(), json!(event.as_str()));
    for (key, value) in fields {
        if REDACTED_FIELDS.contains(key) {
            continue;
        }
        entry.insert((*key).to_string(), value.clone());
    }
    let line = serde_json::to_string(&Value::Object(entry)).unwrap_or_default() + "\n";
    let path = resolve_log_path();
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        f.write_all(line.as_bytes())
    })();
    if let Err(e) = result {
        log::warn!("dashboard-auth audit log write failed: {e}");
    }
}
