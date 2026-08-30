//! Parity tests for `hermes_cli/dashboard_auth/ws_tickets.py` @ b9aa928.
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use std::sync::Mutex;

// The ticket store is process-global; tests are serialized (workspace
// convention for global state).
static STATE_LOCK: Mutex<()> = Mutex::new(());

use hermes_cli::dashboard_auth::ws_tickets::{
    consume_internal_credential, consume_ticket, consume_ticket_at, internal_ws_credential,
    mint_ticket_at, reset_for_tests, TTL_SECONDS,
};

#[test]
fn ticket_round_trip_single_use() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let ticket = mint_ticket_at("u1", "nous", 1_000_000);
    // base64url of 32 bytes = 43 chars, no padding.
    assert_eq!(ticket.len(), 43);
    assert!(!ticket.contains('+') && !ticket.contains('/'));

    let info = consume_ticket_at(&ticket, 1_000_000).unwrap();
    assert_eq!(info["user_id"], "u1");
    assert_eq!(info["provider"], "nous");
    assert_eq!(info["minted_at"], 1_000_000);

    // Single use: second consume is an unknown-ticket error with the value
    // truncated (misuse never logs the secret in full).
    let err = consume_ticket(&ticket).unwrap_err();
    assert!(
        matches!(err, hermes_cli::dashboard_auth::ws_tickets::TicketInvalid::UnknownTicket(ref t)
            if t.starts_with(&ticket[..8]) && t.ends_with('\u{2026}')),
        "{err:?}"
    );
}

#[test]
fn ttl_is_thirty_seconds_with_boundaries() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let now = 1_000_000i64;
    let ticket = mint_ticket_at("u", "nous", now);
    // At TTL-1 still valid.
    let ok = consume_ticket_at(&ticket, now + TTL_SECONDS - 1);
    assert!(ok.is_ok());
    reset_for_tests();
    let ticket = mint_ticket_at("u", "nous", now);
    // At exactly TTL the `expires_at < now` check passes (not yet expired).
    let ok = consume_ticket_at(&ticket, now + TTL_SECONDS);
    assert!(ok.is_ok());
    reset_for_tests();
    let ticket = mint_ticket_at("u", "nous", now);
    let err = consume_ticket_at(&ticket, now + TTL_SECONDS + 1).unwrap_err();
    assert_eq!(
        err,
        hermes_cli::dashboard_auth::ws_tickets::TicketInvalid::Expired
    );
}

#[test]
fn empty_ticket_error_names_the_empty_marker() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let err = consume_ticket("").unwrap_err();
    assert_eq!(
        err,
        hermes_cli::dashboard_auth::ws_tickets::TicketInvalid::UnknownTicket("<empty>".to_string())
    );
}

#[test]
fn internal_credential_is_stable_multi_use_and_never_expires() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let first = internal_ws_credential();
    assert_eq!(first.len(), 43);
    assert_eq!(internal_ws_credential(), first, "minted once per process");

    let info = consume_internal_credential(&first).unwrap();
    assert_eq!(info["user_id"], "server-internal");
    assert_eq!(info["provider"], "server-internal");
    // Multi-use: a second consume succeeds too.
    assert!(consume_internal_credential(&first).is_ok());

    // Wrong value -> mismatch.
    let err = consume_internal_credential("not-the-credential").unwrap_err();
    assert_eq!(
        err,
        hermes_cli::dashboard_auth::ws_tickets::TicketInvalid::InternalCredentialMismatch
    );
    // Empty value -> "no internal credential" arm shape.
    assert!(consume_internal_credential("").is_err());
}

#[test]
fn internal_credential_not_issued_means_reject() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    // No credential minted yet: any value is rejected.
    assert!(consume_internal_credential("guess").is_err());
    // Minting then consuming works again.
    let cred = internal_ws_credential();
    assert!(consume_internal_credential(&cred).is_ok());
}
