//! Parity tests for `hermes_cli/dashboard_auth/native_flow.py` @ 5d59366.
//! Store-level cases derive from the module as oracle (the upstream
//! `test_dashboard_auth_native_flow.py` exercises the HTTP routes,
//! which belong to the web-server surface).

use std::sync::Mutex;

use hermes_cli::dashboard_auth::base::Session;
use hermes_cli::dashboard_auth::native_flow::{
    complete_pending, get_pending, redeem_code, register_pending, reset_for_tests, s256,
    CODE_TTL_SECONDS, MAX_ENTRIES, MAX_PENDING_PER_IP, PENDING_TTL_SECONDS,
};

static STATE_LOCK: Mutex<()> = Mutex::new(());

fn session(name: &str) -> Session {
    Session {
        user_id: "u1".to_string(),
        email: "u1@example.com".to_string(),
        display_name: name.to_string(),
        org_id: String::new(),
        provider: "nous".to_string(),
        expires_at: 1_800_000_000,
        access_token: "at".to_string(),
        refresh_token: "rt".to_string(),
    }
}

fn cv_cc(seed: &str) -> (String, String) {
    // The desktop generates the verifier and derives the S256 challenge.
    (
        format!("verifier-{seed}"),
        s256(&format!("verifier-{seed}")),
    )
}

// ── PKCE helpers ─────────────────────────────────────────────────────────

#[test]
fn s256_matches_rfc7636_sample() {
    // RFC 7636 appendix B: verifier "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    // -> "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".
    assert_eq!(
        s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

// ── full brokered flow ───────────────────────────────────────────────────

#[test]
fn full_flow_register_complete_redeem() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (cv, cc) = cv_cc("a");
    let now = 1_000_000i64;

    let broker_state = register_pending(
        &cc,
        "http://127.0.0.1:8765/callback",
        "client-state-1",
        "10.0.0.1",
        now,
    )
    .unwrap();
    assert_eq!(broker_state.len(), 43, "43-char base64url handle");

    // Read-only peek: the entry survives for the callback.
    let pending = get_pending(&broker_state, now).unwrap();
    assert_eq!(pending.client_state, "client-state-1");
    assert_eq!(pending.redirect_uri, "http://127.0.0.1:8765/callback");

    let gw_code = complete_pending(&broker_state, &session("Desktop"), now).unwrap();
    assert_eq!(gw_code.len(), 43);
    // Pending entry was consumed by complete_pending.
    assert!(get_pending(&broker_state, now).is_err());

    let session = redeem_code(&gw_code, &cv, now).unwrap();
    assert_eq!(session.display_name, "Desktop");

    // Single use: replay finds nothing.
    assert_eq!(
        redeem_code(&gw_code, &cv, now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::CodeInvalid
    );
}

#[test]
fn wrong_verifier_consumes_the_code_no_retry() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (cv, cc) = cv_cc("b");
    let now = 1_000_000i64;
    let broker_state = register_pending(&cc, "http://127.0.0.1:1/cb", "st", "", now).unwrap();
    let gw_code = complete_pending(&broker_state, &session("D"), now).unwrap();

    // Wrong verifier: code already popped, PKCE fails.
    assert_eq!(
        redeem_code(&gw_code, "wrong-verifier", now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::PkceFailed
    );
    // Retry with the RIGHT verifier still fails — no replay.
    assert_eq!(
        redeem_code(&gw_code, &cv, now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::CodeInvalid
    );
}

#[test]
fn ttl_boundaries_for_pending_and_codes() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (cv, cc) = cv_cc("c");
    let now = 2_000_000i64;
    let broker_state = register_pending(&cc, "http://127.0.0.1:1/cb", "st", "", now).unwrap();
    // Pending valid at TTL.
    assert!(get_pending(&broker_state, now + PENDING_TTL_SECONDS).is_ok());
    // complete at the TTL boundary, redeem at the code TTL boundary.
    let gw_code =
        complete_pending(&broker_state, &session("D"), now + PENDING_TTL_SECONDS).unwrap();
    assert!(redeem_code(&gw_code, &cv, now + PENDING_TTL_SECONDS + CODE_TTL_SECONDS).is_ok());

    // Pending expired after TTL: the GC inside get_pending drops the entry.
    let broker_state = register_pending(&cc, "http://127.0.0.1:1/cb", "st", "", now).unwrap();
    assert_eq!(
        get_pending(&broker_state, now + PENDING_TTL_SECONDS + 1).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::PendingNotFound
    );

    reset_for_tests();
    let broker_state = register_pending(&cc, "u", "st", "", now).unwrap();
    let gw_code = complete_pending(&broker_state, &session("D"), now).unwrap();
    // GC runs before lookup, so an expired code reports CodeInvalid (the
    // CodeExpired arm is reachable only for entries inserted post-GC —
    // dead in practice; verified against the Python oracle).
    let err = redeem_code(&gw_code, &cv, now + CODE_TTL_SECONDS + 1).unwrap_err();
    assert_eq!(
        err,
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::CodeInvalid
    );
}

#[test]
fn complete_unknown_broker_state_fails_closed() {
    // `complete_pending` pops (consumes) before minting: an unknown
    // broker_state is PendingNotFound, never a code.
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    assert_eq!(
        complete_pending("never-minted", &session("D"), 1_000_000).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::PendingNotFound
    );
}

#[test]
fn complete_consumes_pending_single_use() {
    // A second complete on the same broker_state finds nothing — the
    // pending entry was consumed by the first.
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (_, cc) = cv_cc("f");
    let now = 5_000_000i64;
    let broker_state = register_pending(&cc, "u", "st", "", now).unwrap();
    complete_pending(&broker_state, &session("D"), now).unwrap();
    assert_eq!(
        complete_pending(&broker_state, &session("D"), now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::PendingNotFound
    );
}

#[test]
fn per_ip_pending_cap_fails_closed() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (_, cc) = cv_cc("d");
    let now = 3_000_000i64;
    for i in 0..MAX_PENDING_PER_IP {
        register_pending(&cc, "u", "st", "10.9.9.9", now + i as i64).unwrap();
    }
    assert_eq!(
        register_pending(&cc, "u", "st", "10.9.9.9", now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::TooManyPendingForAddress
    );
    // A different address is unaffected.
    assert!(register_pending(&cc, "u", "st", "10.9.9.8", now).is_ok());
    // Empty IP (unknown requester) is exempt from the per-IP cap.
    assert!(register_pending(&cc, "u", "st", "", now).is_ok());
}

#[test]
fn global_capacity_cap() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let (_, cc) = cv_cc("e");
    let now = 4_000_000i64;
    // Fill to MAX_ENTRIES with pending entries.
    for i in 0..MAX_ENTRIES {
        register_pending(&cc, "u", "st", "", now + i as i64).unwrap();
    }
    assert_eq!(
        register_pending(&cc, "u", "st", "", now).unwrap_err(),
        hermes_cli::dashboard_auth::native_flow::NativeFlowError::AtCapacity
    );
    // GC: entries expire, capacity frees up.
    assert!(register_pending(&cc, "u", "st", "", now + PENDING_TTL_SECONDS + 1).is_ok());
}
