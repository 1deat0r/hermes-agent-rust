//! Parity tests for `hermes_cli/dashboard_auth/ws_tickets.py` @ 5d59366.
//! Mirrors `tests/hermes_cli/test_dashboard_auth_ws_tickets.py`
//! case-for-case (happy path, single-use, TTL, truncation, concurrency,
//! internal credential).
//!
//! The ticket store is process-global; tests serialize behind a mutex
//! per the workspace convention.

use std::sync::Mutex;

// The ticket store is process-global; tests are serialized (workspace
// convention for global state).
static STATE_LOCK: Mutex<()> = Mutex::new(());

use hermes_cli::dashboard_auth::ws_tickets::{
    consume_internal_credential, consume_ticket, consume_ticket_at, internal_ws_credential,
    mint_ticket, mint_ticket_at, reset_for_tests, INTERNAL_PROVIDER, INTERNAL_USER_ID, TTL_SECONDS,
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

/// PARITY: `TestMintAndConsume::test_round_trip` — info carries the
/// identity triple back to the WS handler.
#[test]
fn mint_consume_round_trip_returns_identity() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let ticket = mint_ticket("u1", "nous");
    let info = consume_ticket(&ticket).unwrap();
    assert_eq!(info["user_id"], "u1");
    assert_eq!(info["provider"], "nous");
    assert!(info.contains_key("minted_at"));
}

/// PARITY: `TestMintAndConsume::test_ticket_has_minimum_length` —
/// `token_urlsafe(32)` entropy floor so a refactor can't shrink it.
#[test]
fn ticket_has_minimum_length() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let ticket = mint_ticket("u1", "nous");
    assert!(ticket.len() >= 32);
}

/// PARITY: `TestSingleUse::test_second_consume_raises` +
/// `test_unknown_ticket_rejected` — consumed/unknown tickets read as
/// "unknown", never as expired.
#[test]
fn second_consume_reads_unknown() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let ticket = mint_ticket("u1", "stub");
    consume_ticket(&ticket).unwrap();
    let err = consume_ticket(&ticket).unwrap_err().to_string();
    assert!(err.contains("unknown"), "{err}");
    let err = consume_ticket("nope-never-minted").unwrap_err().to_string();
    assert!(err.contains("unknown"), "{err}");
}

/// PARITY: `TestTTL::test_constant_is_30_seconds` — pinned so a
/// lifetime change surfaces here.
#[test]
fn ttl_constant_is_thirty_seconds() {
    assert_eq!(TTL_SECONDS, 30);
}

/// PARITY: `TestTTL::test_expired_ticket_rejected` — the explicit-clock
/// forms are the Rust equivalent of patching `ws_tickets.time.time`.
#[test]
fn expired_ticket_rejected() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let now = 1_000_000i64;
    let ticket = mint_ticket_at("u1", "stub", now);
    let err = consume_ticket_at(&ticket, now + TTL_SECONDS + 1).unwrap_err();
    assert_eq!(
        err,
        hermes_cli::dashboard_auth::ws_tickets::TicketInvalid::Expired
    );
}

/// PARITY: `TestErrorMessages::test_unknown_ticket_error_truncates_value`
/// — never more than the first 8 chars of an opaque ticket.
#[test]
fn unknown_ticket_error_truncates_value() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let long_value = "a".repeat(100);
    let message = consume_ticket(&long_value).unwrap_err().to_string();
    assert!(!message.contains(&long_value));
    assert!(message.contains(&long_value[..8]));
}

/// PARITY: `TestConcurrency::test_mint_and_consume_concurrent` — 20
/// threads mint+consume without deadlock or cross-thread bleed.
#[test]
fn mint_and_consume_concurrent() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let handles: Vec<_> = (0..20)
        .map(|i| {
            std::thread::spawn(move || {
                let ticket = mint_ticket(&format!("u{i}"), "stub");
                let info = consume_ticket(&ticket).expect("own ticket consumes");
                info["user_id"].clone()
            })
        })
        .collect();
    let users: std::collections::HashSet<String> = handles
        .into_iter()
        .map(|h| {
            h.join()
                .expect("no deadlock")
                .as_str()
                .expect("user id")
                .to_string()
        })
        .collect();
    let expected: std::collections::HashSet<String> = (0..20).map(|i| format!("u{i}")).collect();
    // Every consume returns a distinct user_id (no cross-thread bleed).
    assert_eq!(users, expected);
}

/// PARITY: `TestInternalCredential::test_reset_clears_and_remints`.
#[test]
fn reset_clears_and_remints() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let first = internal_ws_credential();
    reset_for_tests();
    assert!(consume_internal_credential(&first).is_err());
    let second = internal_ws_credential();
    assert_ne!(second, first);
    assert_eq!(
        consume_internal_credential(&second).unwrap()["user_id"],
        INTERNAL_USER_ID
    );
}

/// PARITY: `TestInternalCredential::test_independent_of_ticket_store`.
#[test]
fn internal_credential_independent_of_ticket_store() {
    let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_tests();
    let cred = internal_ws_credential();
    let ticket = mint_ticket("u1", "nous");
    consume_internal_credential(&cred).unwrap();
    assert_eq!(consume_ticket(&ticket).unwrap()["user_id"], "u1");
    assert_eq!(INTERNAL_PROVIDER, "server-internal");
}
