//! Dashboard-mediated callback bridge for MCP OAuth.
//!
//! PARITY: `tools/mcp_dashboard_oauth.py` @ 5d59366 (whole module,
//! 140 lines). `iss` (RFC 9207) rides the callback into the redeemed
//! triple; blank `state` reads as missing and `%XX`/`+` decode exactly
//! like `parse_qs` (verified against live Python on 6 probes).
//!
//! The MCP SDK remains responsible for discovery, DCR, PKCE, state
//! validation and token exchange. This module only moves the two
//! human/browser callbacks from a loopback listener into the
//! already-authenticated dashboard session.
//!
//! TRANSLATION NOTES: upstream's `asyncio.to_thread(event.wait, timeout)`
//! waits become direct `Condvar::wait_timeout` calls (same bounded-wait
//! semantics, no thread hop needed); the `contextvars.ContextVar` current-
//! flow becomes a thread-local slot (crate convention); the async methods
//! that never await are plain methods here.
//!
//! Error mapping: Python `ValueError` / `RuntimeError` /
//! `TimeoutError` → [`FlowError`] variants `StateMismatch` / `AlreadyEnded`
//! / `CallbackError` / `TimedOut`, preserving the message text.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

/// PARITY: `DashboardOAuthFlow` public dataclass fields (upstream lines
/// 22-34).
#[derive(Debug, Clone)]
pub struct DashboardOAuthFlow {
    pub flow_id: String,
    pub server_name: String,
    pub profile: Option<String>,
    pub hermes_home: String,
    pub redirect_uri: String,
    pub reconnect_live: bool,
    /// PARITY: `created_at: float = field(default_factory=time.time)`.
    pub created_at: f64,
    /// `"starting"` | `"authorization_required"` | `"approved"` | `"error"`.
    pub status: String,
    pub authorization_url: Option<String>,
    pub error: Option<String>,
    pub tools: Vec<Value>,
}

impl DashboardOAuthFlow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        flow_id: &str,
        server_name: &str,
        profile: Option<&str>,
        hermes_home: &str,
        redirect_uri: &str,
        reconnect_live: bool,
    ) -> Self {
        Self {
            flow_id: flow_id.to_string(),
            server_name: server_name.to_string(),
            profile: profile.map(str::to_string),
            hermes_home: hermes_home.to_string(),
            redirect_uri: redirect_uri.to_string(),
            reconnect_live,
            created_at: now_unix_f64(),
            status: "starting".to_string(),
            authorization_url: None,
            error: None,
            tools: Vec::new(),
        }
    }
}

fn now_unix_f64() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Errors mirroring the upstream `ValueError` / `RuntimeError` /
/// `TimeoutError` raises, with the message text preserved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FlowError {
    #[error("OAuth authorization URL did not include state")]
    MissingState,
    #[error("OAuth flow already ended")]
    AlreadyEnded,
    #[error("OAuth callback already received")]
    CallbackAlreadyReceived,
    #[error("OAuth callback state mismatch")]
    StateMismatch,
    #[error("OAuth callback did not include code or error")]
    MissingCodeOrError,
    #[error("Timed out waiting for MCP authorization URL")]
    TimedOutWaitingForAuthorizationUrl,
    #[error("Timed out waiting for MCP OAuth callback")]
    TimedOutWaitingForCallback,
    #[error("MCP OAuth flow ended before authorization: {0}")]
    EndedBeforeAuthorization(String),
    #[error("OAuth authorization failed: {0}")]
    AuthorizationFailed(String),
    #[error("OAuth callback did not include an authorization code")]
    MissingAuthorizationCode,
}

struct Inner {
    expected_state: Option<String>,
    callback: Option<(String, Option<String>, Option<String>)>,
    callback_error: Option<String>,
    authorization_ready: bool,
    callback_ready: bool,
    worker_done: bool,
}

/// PARITY: the `DashboardOAuthFlow` mutable/locked halves (upstream lines
/// 35-133) as one handleable state machine.
#[derive(Clone)]
pub struct DashboardOAuthFlowHandle {
    flow: Arc<Mutex<DashboardOAuthFlow>>,
    inner: Arc<Mutex<Inner>>,
    signal: Arc<std::sync::Condvar>,
}

impl DashboardOAuthFlowHandle {
    pub fn new(flow: DashboardOAuthFlow) -> Self {
        Self {
            flow: Arc::new(Mutex::new(flow)),
            inner: Arc::new(Mutex::new(Inner {
                expected_state: None,
                callback: None,
                callback_error: None,
                authorization_ready: false,
                callback_ready: false,
                worker_done: false,
            })),
            signal: Arc::new(std::sync::Condvar::new()),
        }
    }

    /// Snapshot the immutable + status fields.
    ///
    /// PARITY: `snapshot` (upstream lines 124-132).
    pub fn snapshot(&self) -> Value {
        let flow = self.flow.lock().unwrap_or_else(|e| e.into_inner());
        json!({
            "flow_id": flow.flow_id,
            "server_name": flow.server_name,
            "status": flow.status,
            "authorization_url": flow.authorization_url,
            "error": flow.error,
        })
    }

    /// Publish the IDP authorization URL, extracting and pinning its
    /// `state` query parameter for callback validation.
    ///
    /// PARITY: `publish_authorization_url` (upstream lines 63-78).
    pub fn publish_authorization_url(&self, url: &str) -> Result<(), FlowError> {
        let state = parse_query_param(url, "state").ok_or(FlowError::MissingState)?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut flow = self.flow.lock().unwrap_or_else(|e| e.into_inner());
        if flow.status == "approved" || flow.status == "error" {
            return Err(FlowError::AlreadyEnded);
        }
        flow.authorization_url = Some(url.to_string());
        inner.expected_state = Some(state);
        flow.status = "authorization_required".to_string();
        inner.authorization_ready = true;
        self.signal.notify_all();
        Ok(())
    }

    /// Wait boundedly for the authorization URL.
    ///
    /// PARITY: `wait_for_authorization_url` (upstream lines 80-86).
    pub fn wait_for_authorization_url(&self, timeout: f64) -> Result<String, FlowError> {
        let deadline = std::time::Instant::now() + Duration::from_secs_f64(timeout);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        while !inner.authorization_ready {
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(FlowError::TimedOutWaitingForAuthorizationUrl);
            }
            let (guard, _) = self
                .signal
                .wait_timeout(inner, deadline - now)
                .unwrap_or_else(|e| e.into_inner());
            inner = guard;
        }
        let flow = self.flow.lock().unwrap_or_else(|e| e.into_inner());
        match &flow.authorization_url {
            Some(url) => Ok(url.clone()),
            None => Err(FlowError::EndedBeforeAuthorization(
                flow.error
                    .clone()
                    .unwrap_or_else(|| "MCP OAuth flow ended before authorization".to_string()),
            )),
        }
    }

    /// Deliver the browser callback (code / state / error triple).
    ///
    /// `iss` (RFC 9207) is carried through into the redeemed triple —
    /// see `tools.mcp_oauth._parse_redirect_query` upstream.
    ///
    /// PARITY: `deliver_callback` (upstream lines 73-91).
    pub fn deliver_callback(
        &self,
        code: Option<&str>,
        state: Option<&str>,
        error: Option<&str>,
        iss: Option<&str>,
    ) -> Result<(), FlowError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.callback_ready {
            return Err(FlowError::CallbackAlreadyReceived);
        }
        let expected_ok = inner
            .expected_state
            .as_ref()
            .zip(state)
            .map(|(expected, state)| constant_time_eq(expected, state))
            .unwrap_or(false);
        if !expected_ok {
            return Err(FlowError::StateMismatch);
        }
        if let Some(error) = error {
            inner.callback_error = Some(error.to_string());
        } else if let Some(code) = code {
            inner.callback = Some((
                code.to_string(),
                state.map(str::to_string),
                iss.map(str::to_string),
            ));
        } else {
            inner.callback_error = Some("OAuth callback did not include code or error".to_string());
        }
        inner.callback_ready = true;
        self.signal.notify_all();
        Ok(())
    }

    /// Wait boundedly for the callback and return `(code, state, iss)`.
    ///
    /// PARITY: `wait_for_callback` (upstream lines 93-100).
    pub fn wait_for_callback(
        &self,
        timeout: f64,
    ) -> Result<(String, Option<String>, Option<String>), FlowError> {
        let deadline = std::time::Instant::now() + Duration::from_secs_f64(timeout);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        while !inner.callback_ready {
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(FlowError::TimedOutWaitingForCallback);
            }
            let (guard, _) = self
                .signal
                .wait_timeout(inner, deadline - now)
                .unwrap_or_else(|e| e.into_inner());
            inner = guard;
        }
        if let Some(error) = &inner.callback_error {
            return Err(FlowError::AuthorizationFailed(error.clone()));
        }
        inner
            .callback
            .clone()
            .ok_or(FlowError::MissingAuthorizationCode)
    }

    /// PARITY: `mark_approved` (upstream lines 122-129).
    pub fn mark_approved(&self) -> Result<(), FlowError> {
        let mut flow = self.flow.lock().unwrap_or_else(|e| e.into_inner());
        if flow.status == "error" {
            return Err(FlowError::AlreadyEnded);
        }
        flow.status = "approved".to_string();
        flow.error = None;
        Ok(())
    }

    /// Mark the flow errored; a no-op when already approved.
    ///
    /// PARITY: `mark_error` (upstream lines 132-141).
    pub fn mark_error(&self, error: &str) {
        let mut flow = self.flow.lock().unwrap_or_else(|e| e.into_inner());
        if flow.status == "approved" {
            return;
        }
        flow.status = "error".to_string();
        flow.error = Some(error.to_string());
        drop(flow);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.authorization_ready = true;
        inner.callback_ready = true;
        self.signal.notify_all();
    }

    /// PARITY: `mark_worker_done` / `worker_done` (upstream lines 143-149).
    pub fn mark_worker_done(&self) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .worker_done = true;
    }

    pub fn worker_done(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .worker_done
    }
}

// PARITY: the module-level `dashboard_oauth_flow` context manager +
// `get_dashboard_oauth_flow` (upstream lines 152-170) — a thread-local
// current-flow slot (the crate convention for ContextVars).
thread_local! {
    static CURRENT_FLOW: RefCell<Option<Arc<DashboardOAuthFlowHandle>>> =
        const { RefCell::new(None) };
}

/// Bind the active flow for this thread; restores the prior value on drop
/// (the context-manager `finally: reset(token)` arm).
pub struct DashboardOAuthFlowGuard {
    previous: Option<Arc<DashboardOAuthFlowHandle>>,
}

impl Drop for DashboardOAuthFlowGuard {
    fn drop(&mut self) {
        CURRENT_FLOW.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

/// PARITY: the `dashboard_oauth_flow(flow)` context manager entry.
pub fn set_dashboard_oauth_flow(flow: Arc<DashboardOAuthFlowHandle>) -> DashboardOAuthFlowGuard {
    CURRENT_FLOW.with(|slot| {
        let previous = slot.borrow_mut().replace(flow);
        DashboardOAuthFlowGuard { previous }
    })
}

/// PARITY: `get_dashboard_oauth_flow` (upstream lines 169-170).
pub fn get_dashboard_oauth_flow() -> Option<Arc<DashboardOAuthFlowHandle>> {
    CURRENT_FLOW.with(|slot| slot.borrow().clone())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// `parse_qs(urlparse(url).query).get("state", [None])[0]` — the first
/// `state` value, or None.
///
/// `parse_qs` drops blank values and percent-decodes (`+` → space), so
/// `?state=` reads as missing and `%XX` sequences decode. Both are
/// matched here.
fn parse_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1.split('#').next()?;
    let mut values: VecDeque<String> = VecDeque::new();
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == key && !v.is_empty() {
            values.push_back(percent_decode(v));
        }
    }
    values.pop_front()
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 {
            let h = hex_val(bytes.get(i + 1).copied().unwrap_or(0));
            let l = hex_val(bytes.get(i + 2).copied().unwrap_or(0));
            if let (Some(h), Some(l)) = (h, l) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
