//! Parity tests for `tools/mcp_dashboard_oauth.py` @ b9aa928. Upstream has
//! no dedicated test file (missing-test gap, noted in the ledger); cases
//! derive from the upstream code as oracle.

use std::sync::Arc;

use hermes_tools::mcp_dashboard_oauth::{
    get_dashboard_oauth_flow, set_dashboard_oauth_flow, DashboardOAuthFlow,
    DashboardOAuthFlowHandle, FlowError,
};

fn make_flow() -> DashboardOAuthFlowHandle {
    DashboardOAuthFlowHandle::new(DashboardOAuthFlow::new(
        "flow-1",
        "context7",
        Some("work"),
        "/home/u/.hermes",
        "http://127.0.0.1:8791/callback",
        false,
    ))
}

#[test]
fn publish_pins_state_and_moves_status() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp/authorize?state=st-9&client_id=x")
        .unwrap();
    assert_eq!(flow.snapshot()["status"], "authorization_required");
    assert_eq!(
        flow.snapshot()["authorization_url"],
        "https://idp/authorize?state=st-9&client_id=x"
    );
    // wait_for_authorization_url now returns immediately with the URL.
    assert_eq!(
        flow.wait_for_authorization_url(1.0).unwrap(),
        "https://idp/authorize?state=st-9&client_id=x"
    );
}

#[test]
fn publish_without_state_is_rejected() {
    let flow = make_flow();
    assert_eq!(
        flow.publish_authorization_url("https://idp/authorize"),
        Err(FlowError::MissingState)
    );
}

#[test]
fn double_publish_after_end_is_rejected() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=a")
        .unwrap();
    flow.mark_error("boom");
    // Ended flow: publishing raises "already ended" upstream.
    assert_eq!(
        flow.publish_authorization_url("https://idp?state=b"),
        Err(FlowError::AlreadyEnded)
    );
}

#[test]
fn callback_round_trip_with_state_validation() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=expected")
        .unwrap();
    flow.deliver_callback(Some("the-code"), Some("expected"), None)
        .unwrap();
    let (code, state) = flow.wait_for_callback(1.0).unwrap();
    assert_eq!(code, "the-code");
    assert_eq!(state.as_deref(), Some("expected"));

    // Second callback rejected ("already received").
    assert_eq!(
        flow.deliver_callback(Some("c2"), Some("expected"), None),
        Err(FlowError::CallbackAlreadyReceived)
    );
}

#[test]
fn state_mismatch_is_rejected() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=expected")
        .unwrap();
    assert_eq!(
        flow.deliver_callback(Some("code"), Some("tampered"), None),
        Err(FlowError::StateMismatch)
    );
    // None state with a pinned expectation also mismatches.
    assert_eq!(
        flow.deliver_callback(Some("code"), None, None),
        Err(FlowError::StateMismatch)
    );
}

#[test]
fn callback_error_surfaces_on_wait() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=s")
        .unwrap();
    flow.deliver_callback(None, Some("s"), Some("user_denied"))
        .unwrap();
    assert_eq!(
        flow.wait_for_callback(1.0),
        Err(FlowError::AuthorizationFailed("user_denied".to_string()))
    );
}

#[test]
fn neither_code_nor_error_is_an_error() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=s")
        .unwrap();
    flow.deliver_callback(None, Some("s"), None).unwrap();
    // wait_for_callback wraps the stored error in the
    // "OAuth authorization failed" raise.
    assert_eq!(
        flow.wait_for_callback(1.0),
        Err(FlowError::AuthorizationFailed(
            "OAuth callback did not include code or error".to_string()
        ))
    );
}

#[test]
fn mark_error_wakes_waiters_and_snapshot_reflects_it() {
    let flow = make_flow();
    flow.mark_error("idp down");
    assert_eq!(flow.snapshot()["status"], "error");
    assert_eq!(flow.snapshot()["error"], "idp down");
    // wait_for_authorization_url raises the flow error instead of timing out.
    assert!(matches!(
        flow.wait_for_authorization_url(1.0),
        Err(FlowError::EndedBeforeAuthorization(_))
    ));
    // mark_approved after error is rejected ("already ended").
    assert_eq!(flow.mark_approved(), Err(FlowError::AlreadyEnded));
}

#[test]
fn mark_error_is_a_noop_after_approval() {
    let flow = make_flow();
    flow.publish_authorization_url("https://idp?state=s")
        .unwrap();
    flow.mark_approved().unwrap();
    assert_eq!(flow.snapshot()["status"], "approved");
    // `if self.status == "approved": return` — the error is dropped.
    flow.mark_error("late error");
    assert_eq!(flow.snapshot()["status"], "approved");
    assert_eq!(flow.snapshot()["error"], serde_json::Value::Null);
}

#[test]
fn worker_done_flag_and_thread_local_flow() {
    let flow = Arc::new(make_flow());
    {
        let _guard = set_dashboard_oauth_flow(Arc::clone(&flow));
        let current = get_dashboard_oauth_flow().unwrap();
        assert_eq!(current.snapshot()["flow_id"], "flow-1");
        flow.mark_worker_done();
        assert!(flow.worker_done());
    }
    // Outside the guard the slot is empty (context-manager reset).
    assert!(get_dashboard_oauth_flow().is_none());
}

#[test]
fn snapshot_fields_match_upstream_keys() {
    let flow = make_flow();
    let snap = flow.snapshot().as_object().unwrap().clone();
    for key in [
        "flow_id",
        "server_name",
        "status",
        "authorization_url",
        "error",
    ] {
        assert!(snap.contains_key(key), "missing {key}");
    }
}
