//! Parity oracles for the compression-lock lifecycle + compression
//! publication/recovery, mirroring upstream tests @ b9aa928:
//!   tests/test_hermes_state.py::test_refresh_compression_lock_*
//!   tests/state/test_compression_lineage_guard.py
//!   tests/test_session_system_prompt_dedup.py::test_compression_child_*
//! Time control in the upstream tests uses monkeypatch (frozen time); here
//! determinism is achieved by manipulating expires_at directly.

use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::state::{SessionDB, WriteError};
use serde_json::json;

fn tmp_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn lock_expires(db: &SessionDB, session_id: &str) -> Option<f64> {
    db.writer_conn()
        .query_row(
            "SELECT expires_at FROM compression_locks WHERE session_id = ?",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .ok()
}

fn lock_holder(db: &SessionDB, session_id: &str) -> Option<String> {
    db.writer_conn()
        .query_row(
            "SELECT holder FROM compression_locks WHERE session_id = ?",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .ok()
}

fn expire_lock(db: &SessionDB, session_id: &str) {
    db.writer_conn()
        .execute(
            "UPDATE compression_locks SET expires_at = 0 WHERE session_id = ?",
            rusqlite::params![session_id],
        )
        .unwrap();
}

fn user_message(content: &str) -> MessageInput {
    MessageInput {
        role: "user".to_string(),
        content: Some(json!(content)),
        ..Default::default()
    }
}

#[test]
fn refresh_requires_holder_and_preserves_reclaimability() {
    // test_refresh_compression_lock_requires_holder_and_preserves_reclaimability
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    assert!(db.try_acquire_compression_lock("s1", "holder-a", 100.0));
    let original = lock_expires(&db, "s1").expect("lock row exists");

    assert!(db.refresh_compression_lock("s1", "holder-a", 100.0));
    let refreshed = lock_expires(&db, "s1").expect("lock row exists");
    assert!(refreshed > original);

    assert!(!db.refresh_compression_lock("s1", "holder-b", 100.0));

    // holder-a's lease lapses; holder-b must be able to reclaim it.
    expire_lock(&db, "s1");
    assert!(db.try_acquire_compression_lock("s1", "holder-b", 100.0));
    db.close();
}

#[test]
fn refresh_cannot_resurrect_a_lock_already_reclaimed() {
    // test_refresh_cannot_resurrect_a_lock_already_reclaimed
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    assert!(db.try_acquire_compression_lock("s1", "holder-a", 100.0));
    expire_lock(&db, "s1");
    assert!(db.try_acquire_compression_lock("s1", "holder-b", 100.0));

    // holder-a coming back late must NOT steal it back.
    assert!(!db.refresh_compression_lock("s1", "holder-a", 100.0));
    assert_eq!(lock_holder(&db, "s1").as_deref(), Some("holder-b"));
    db.close();
}

#[test]
fn acquire_reclaims_dead_structured_holder_immediately() {
    // A gateway killed during compression must not stall the replacement for
    // the full TTL: a holder carrying a gone local pid is reclaimed now.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    db.writer_conn()
        .execute(
            "INSERT INTO compression_locks (session_id, holder, acquired_at, expires_at) \
             VALUES ('s1', 'pid=99999999:1:deadbeef', 0, ?)",
            rusqlite::params![hermes_state::state::now() + 1000.0],
        )
        .unwrap();
    assert!(db.try_acquire_compression_lock("s1", "holder-x", 100.0));
    assert_eq!(lock_holder(&db, "s1").as_deref(), Some("holder-x"));
    db.close();
}

#[test]
fn acquire_never_reclaims_same_process_or_unstructured_holder() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    // Same-process holder (this test process owns the pid): never reclaimed.
    let pid = std::process::id();
    db.writer_conn()
        .execute(
            "INSERT INTO compression_locks (session_id, holder, acquired_at, expires_at) \
             VALUES ('s1', ?, 0, ?)",
            rusqlite::params![format!("pid={}:1:live", pid), hermes_state::state::now() + 1000.0],
        )
        .unwrap();
    assert!(!db.try_acquire_compression_lock("s1", "competitor", 100.0));
    assert!(lock_holder(&db, "s1").unwrap().starts_with(&format!("pid={}", pid)));

    // Unstructured holder (no pid= marker): TTL-only, never reclaimed.
    db.writer_conn()
        .execute(
            "INSERT INTO compression_locks (session_id, holder, acquired_at, expires_at) \
             VALUES ('s2', 'legacy-holder-name', 0, ?)",
            rusqlite::params![hermes_state::state::now() + 1000.0],
        )
        .unwrap();
    assert!(!db.try_acquire_compression_lock("s2", "competitor", 100.0));
    assert_eq!(lock_holder(&db, "s2").as_deref(), Some("legacy-holder-name"));
    db.close();
}

#[test]
fn release_is_idempotent_and_holder_checked() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    assert!(db.try_acquire_compression_lock("s1", "owner", 100.0));
    // Wrong holder cannot clobber.
    db.release_compression_lock("s1", "intruder");
    assert_eq!(lock_holder(&db, "s1").as_deref(), Some("owner"));
    // Owner releases.
    db.release_compression_lock("s1", "owner");
    assert!(lock_holder(&db, "s1").is_none());
    // Release when no lock exists is a no-op.
    db.release_compression_lock("s1", "owner");
    db.close();
}

#[test]
fn get_holder_returns_only_non_expired() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();

    assert!(db.try_acquire_compression_lock("s1", "owner", 100.0));
    assert_eq!(db.get_compression_lock_holder("s1").as_deref(), Some("owner"));
    expire_lock(&db, "s1");
    assert_eq!(db.get_compression_lock_holder("s1"), None);
    // Fast path: empty session id.
    assert_eq!(db.get_compression_lock_holder(""), None);
    db.close();
}

// ── find_live_compression_child ─────────────────────────────────────────────

fn compression_parent(db: &SessionDB, session_id: &str) {
    db.create_session(session_id, "webui", &NewSession::default()).unwrap();
    db.append_message(session_id, &user_message("before split"), None).unwrap();
    db.end_session(session_id, "compression").unwrap();
}

#[test]
fn find_live_compression_child_returns_unique_direct_child() {
    // tests/state/test_compression_lineage_guard.py::test_find_live_...
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "parent");
    db.create_session("child", "webui", &NewSession {
        parent_session_id: Some("parent".to_string()),
        ..Default::default()
    })
    .unwrap();

    let child = db.find_live_compression_child("parent").unwrap().unwrap();
    assert_eq!(child.id, "child");
    assert_eq!(child.parent_session_id.as_deref(), Some("parent"));
    assert!(child.ended_at.is_none());
    db.close();
}

#[test]
fn find_live_compression_child_fails_closed_when_ambiguous() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "parent");
    db.create_session("child-a", "webui", &NewSession {
        parent_session_id: Some("parent".to_string()),
        ..Default::default()
    })
    .unwrap();
    db.create_session("child-b", "webui", &NewSession {
        parent_session_id: Some("parent".to_string()),
        ..Default::default()
    })
    .unwrap();

    assert!(db.find_live_compression_child("parent").unwrap().is_none());
    // Non-compression parent and empty id resolve to None.
    db.create_session("live", "cli", &NewSession::default()).unwrap();
    assert!(db.find_live_compression_child("live").unwrap().is_none());
    assert!(db.find_live_compression_child("").unwrap().is_none());
    db.close();
}

#[test]
fn reopen_orphaned_compression_session_reopens_parent_without_child() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "orphan");

    assert!(db.reopen_orphaned_compression_session("orphan").unwrap());
    let row = db.get_session("orphan").unwrap().unwrap();
    assert!(row.ended_at.is_none());
    assert!(row.end_reason.is_none());

    db.append_message("orphan", &user_message("recovered turn"), None).unwrap();
    let msgs = db.get_messages("orphan", false, None, 0).unwrap();
    let contents: Vec<Option<serde_json::Value>> =
        msgs.iter().map(|m| m.content.clone()).collect();
    assert_eq!(contents, vec![Some(json!("before split")), Some(json!("recovered turn"))]);
    db.close();
}

#[test]
fn reopen_orphaned_compression_session_fails_closed_with_child() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "parent-with-child");
    db.create_session("child", "webui", &NewSession {
        parent_session_id: Some("parent-with-child".to_string()),
        ..Default::default()
    })
    .unwrap();

    assert!(!db.reopen_orphaned_compression_session("parent-with-child").unwrap());
    let parent = db.get_session("parent-with-child").unwrap().unwrap();
    assert_eq!(parent.end_reason.as_deref(), Some("compression"));
    assert!(parent.ended_at.is_some());
    db.close();
}

#[test]
fn reopen_orphaned_compression_session_ignores_non_continuation_children() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "parent-with-branch");
    db.create_session("branch", "webui", &NewSession {
        parent_session_id: Some("parent-with-branch".to_string()),
        model_config: Some(json!({"_branched_from": "parent-with-branch"})),
        ..Default::default()
    })
    .unwrap();
    db.create_session("delegate", "tool", &NewSession {
        parent_session_id: Some("parent-with-branch".to_string()),
        model_config: Some(json!({"_delegate_from": "parent-with-branch"})),
        ..Default::default()
    })
    .unwrap();

    assert!(db.reopen_orphaned_compression_session("parent-with-branch").unwrap());
    db.close();
}

#[test]
fn reopen_fails_closed_when_continuation_inherits_foreign_markers() {
    // A REAL continuation can carry _delegate_from/_branched_from pointing at
    // some OTHER session. Markers only disqualify a child when they point at
    // the queried parent (upstream comment + regression).
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "delegate-session");
    db.create_session("delegate-continuation", "subagent", &NewSession {
        parent_session_id: Some("delegate-session".to_string()),
        model_config: Some(json!({"_delegate_from": "some-original-parent"})),
        ..Default::default()
    })
    .unwrap();

    assert!(!db.reopen_orphaned_compression_session("delegate-session").unwrap());
    let parent = db.get_session("delegate-session").unwrap().unwrap();
    assert_eq!(parent.end_reason.as_deref(), Some("compression"));
    db.close();
}

#[test]
fn reopen_fails_closed_while_lease_active_or_already_open() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    compression_parent(&db, "leased");
    assert!(db.try_acquire_compression_lock("leased", "owner", 1000.0));
    assert!(!db.reopen_orphaned_compression_session("leased").unwrap());
    // A non-compression-ended session is not reopenable.
    db.create_session("open-session", "cli", &NewSession::default()).unwrap();
    assert!(!db.reopen_orphaned_compression_session("open-session").unwrap());
    db.close();
}

// ── publish_compression_child ───────────────────────────────────────────────

#[test]
fn publish_compression_child_uses_content_addressed_prompt() {
    // test_session_system_prompt_dedup.py::test_compression_child_uses_...
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let prompt = "compressed child prompt";
    db.create_session("parent", "webui", &NewSession::default()).unwrap();
    db.append_message("parent", &user_message("original"), None).unwrap();
    assert!(db.try_acquire_compression_lock("parent", "holder", 60.0));

    db.publish_compression_child(
        "parent",
        "child",
        "webui",
        &[user_message("summary")],
        None,
        None,
        Some(prompt),
        None,
        None,
        Some("holder"),
        true,
    )
    .unwrap();

    let raw = db
        .writer_conn()
        .query_row(
            "SELECT system_prompt, system_prompt_hash FROM sessions WHERE id = 'child'",
            [],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .unwrap();
    assert!(raw.0.is_none());
    assert!(raw.1.is_some());
    assert_eq!(db.get_session("child").unwrap().unwrap().system_prompt.as_deref(), Some(prompt));
    // Parent closed atomically with child publication.
    let parent = db.get_session("parent").unwrap().unwrap();
    assert_eq!(parent.end_reason.as_deref(), Some("compression"));
    assert!(parent.ended_at.is_some());
    // Child inherits parent origin/profile columns.
    let child = db.get_session("child").unwrap().unwrap();
    assert_eq!(child.parent_session_id.as_deref(), Some("parent"));
    assert_eq!(child.message_count, 1);
    db.close();
}

#[test]
fn publish_requires_lease_and_rejects_bad_states() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("parent", "webui", &NewSession::default()).unwrap();

    // No lease at all -> CompressionSessionBusyError.
    let err = db
        .publish_compression_child(
            "parent", "child", "webui", &[user_message("x")], None, None, None, None, None,
            None, true,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::CompressionBusy(_)), "got {:?}", err);

    // Wrong holder -> busy.
    db.try_acquire_compression_lock("parent", "real-holder", 100.0);
    let err = db
        .publish_compression_child(
            "parent", "child", "webui", &[user_message("x")], None, None, None, None, None,
            Some("imposter"), true,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::CompressionBusy(_)), "got {:?}", err);

    // Unknown parent: upstream checks the lease FIRST, so with a required
    // lease the missing lock row is a busy error before the parent probe.
    let err = db
        .publish_compression_child(
            "ghost", "child", "webui", &[user_message("x")], None, None, None, None, None,
            Some("real-holder"), true,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::CompressionBusy(_)), "got {:?}", err);
    // With require_compression_lease=false the parent probe runs: RuntimeError.
    let err = db
        .publish_compression_child(
            "ghost", "child", "webui", &[user_message("x")], None, None, None, None, None,
            None, false,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::Runtime(_)), "got {:?}", err);

    // Empty handoff -> RuntimeError ("must not be empty").
    let err = db
        .publish_compression_child(
            "parent", "child", "webui", &[], None, None, None, None, None,
            Some("real-holder"), true,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::Runtime(_)), "got {:?}", err);

    // require_compression_lease=false allows publication without a lock.
    db.publish_compression_child(
        "parent", "child2", "webui", &[user_message("x")], None, None, None, None, None,
        None, false,
    )
    .unwrap();
    // Parent now ended -> publishing again fails with RuntimeError.
    let err = db
        .publish_compression_child(
            "parent", "child3", "webui", &[user_message("x")], None, None, None, None, None,
            Some("real-holder"), true,
        )
        .unwrap_err();
    assert!(matches!(err, WriteError::Runtime(_)), "got {:?}", err);
    db.close();
}
