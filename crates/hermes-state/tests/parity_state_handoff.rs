//! Parity oracles for the cross-platform session handoff state machine,
//! mirroring upstream tests/hermes_cli/test_session_handoff.py
//! (TestHandoffStateDB) @ b9aa928. Slash-command registration tests land
//! with the P2 CLI crate.

use std::path::PathBuf;

use hermes_state::crud::NewSession;
use hermes_state::state::SessionDB;

fn tmp_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn make_session(db: &SessionDB, sid: &str, source: &str) {
    db.create_session(sid, source, &NewSession::default())
        .expect("create");
}

#[test]
fn list_pending_excludes_running_and_terminal() {
    let (_dir, db) = tmp_db_owned();
    for sid in ["sess-a", "sess-b", "sess-c", "sess-d"] {
        make_session(&db, sid, "cli");
    }
    assert!(db.request_handoff("sess-a", "telegram").expect("req"));
    assert!(db.request_handoff("sess-b", "discord").expect("req"));
    assert!(db.request_handoff("sess-c", "telegram").expect("req"));
    assert!(db.claim_handoff("sess-c").expect("claim")); // running
    assert!(db.request_handoff("sess-d", "slack").expect("req"));
    assert!(db.claim_handoff("sess-d").expect("claim"));
    db.complete_handoff("sess-d").expect("complete"); // terminal

    let pending = db.list_pending_handoffs();
    let ids: Vec<&str> = pending.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec!["sess-a", "sess-b"]);
    db.close();
}

#[test]
fn request_handoff_rejects_in_flight() {
    let (_dir, db) = tmp_db_owned();
    make_session(&db, "sess-x", "cli");
    assert!(db.request_handoff("sess-x", "telegram").expect("req"));
    // Already pending → False (in flight).
    assert!(!db.request_handoff("sess-x", "discord").expect("req"));
    assert!(db.claim_handoff("sess-x").expect("claim"));
    // Running → False.
    assert!(!db.request_handoff("sess-x", "discord").expect("req"));
    // Terminal states allow re-request.
    db.complete_handoff("sess-x").expect("complete");
    assert!(db.request_handoff("sess-x", "slack").expect("req"));
    // Missing row → False.
    assert!(!db.request_handoff("ghost", "telegram").expect("req"));
    db.close();
}

#[test]
fn complete_handoff_clears_error() {
    let (_dir, db) = tmp_db_owned();
    make_session(&db, "sess-complete", "cli");
    db.request_handoff("sess-complete", "telegram")
        .expect("req");
    db.claim_handoff("sess-complete").expect("claim");
    db.fail_handoff("sess-complete", "transient").expect("fail");

    let state = db.get_handoff_state("sess-complete").expect("state");
    assert_eq!(state.state.as_deref(), Some("failed"));
    assert_eq!(state.error.as_deref(), Some("transient"));

    // User retries: failed → pending is allowed.
    db.request_handoff("sess-complete", "telegram")
        .expect("req");
    db.claim_handoff("sess-complete").expect("claim");
    db.complete_handoff("sess-complete").expect("complete");

    let state = db.get_handoff_state("sess-complete").expect("state");
    assert_eq!(state.state.as_deref(), Some("completed"));
    assert_eq!(state.error, None);
    db.close();
}

#[test]
fn full_pending_to_completed_flow() {
    let (_dir, db) = tmp_db_owned();
    make_session(&db, "sess-flow", "cli");

    // CLI: request handoff.
    assert!(db.request_handoff("sess-flow", "telegram").expect("req"));
    assert_eq!(
        db.get_handoff_state("sess-flow")
            .expect("s")
            .state
            .as_deref(),
        Some("pending")
    );

    // Gateway watcher: discover + claim.
    let pending = db.list_pending_handoffs();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["id"].as_str(), Some("sess-flow"));
    assert!(db.claim_handoff("sess-flow").expect("claim"));
    assert_eq!(
        db.get_handoff_state("sess-flow")
            .expect("s")
            .state
            .as_deref(),
        Some("running")
    );

    // Terminal.
    db.complete_handoff("sess-flow").expect("complete");
    assert_eq!(
        db.get_handoff_state("sess-flow")
            .expect("s")
            .state
            .as_deref(),
        Some("completed")
    );
    assert!(db.list_pending_handoffs().is_empty());
    db.close();
}

#[test]
fn handoff_state_none_for_missing_row() {
    let (_dir, db) = tmp_db_owned();
    make_session(&db, "sess-known", "cli");
    assert!(db.get_handoff_state("sess-unknown").is_none());
    let s = db.get_handoff_state("sess-known").expect("state");
    assert!(s.state.is_none() && s.platform.is_none() && s.error.is_none());
    db.close();
}

#[test]
fn fail_handoff_truncates_error_at_500_chars() {
    let (_dir, db) = tmp_db_owned();
    make_session(&db, "sess-long", "cli");
    db.request_handoff("sess-long", "telegram").expect("req");
    db.claim_handoff("sess-long").expect("claim");
    let long: String = "x".repeat(1000);
    db.fail_handoff("sess-long", &long).expect("fail");
    let state = db.get_handoff_state("sess-long").expect("state");
    assert_eq!(state.error.as_deref().map(|s| s.chars().count()), Some(500));
    db.close();
}

fn tmp_db_owned() -> (tempfile::TempDir, SessionDB) {
    let (dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    (dir, db)
}
