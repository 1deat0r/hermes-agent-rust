//! Parity oracles for the compression-failure cooldown + anti-thrash counter
//! surface, mirroring upstream tests/agent/test_compression_concurrent_fork.py
//! (restore-exact-row semantics), tests/agent/test_compaction_anti_thrash.py
//! and test_compression_anti_thrash_recovery.py (streak/count persistence)
//! @ b9aa928. Agent-level orchestration is deferred to P2; these pin the
//! SessionDB contract.

use std::path::PathBuf;

use hermes_state::crud::NewSession;
use hermes_state::state::{now, SessionDB, WriteError};
use serde_json::{json, Value};

fn tmp_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn open_db(name: &str) -> (tempfile::TempDir, SessionDB) {
    let (dir, path) = tmp_db(name);
    let db = SessionDB::open(Some(path), false).expect("open");
    (dir, db)
}

fn cooldown_cols(db: &SessionDB, sid: &str) -> (Option<f64>, Option<String>) {
    let conn = db.writer_conn();
    conn.query_row(
        "SELECT compression_failure_cooldown_until, compression_failure_error \
         FROM sessions WHERE id = ?",
        rusqlite::params![sid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .expect("cols")
}

#[test]
fn record_and_get_active_cooldown() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let until = now() + 120.0;
    db.record_compression_failure_cooldown("s1", until, Some("summarizer boom"));

    let cd = db.get_compression_failure_cooldown("s1").expect("get").expect("some");
    let o = cd.as_object().unwrap();
    assert_eq!(o["cooldown_until"].as_f64(), Some(until));
    assert!(o["remaining_seconds"].as_f64().unwrap() > 0.0);
    assert!(o["remaining_seconds"].as_f64().unwrap() <= 120.0);
    assert_eq!(o["error"].as_str(), Some("summarizer boom"));
    // Raw column persisted verbatim.
    let (raw_until, raw_err) = cooldown_cols(&db, "s1");
    assert_eq!(raw_until, Some(until));
    assert_eq!(raw_err.as_deref(), Some("summarizer boom"));
}

#[test]
fn expired_cooldown_is_inactive_but_row_preserved() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let until = now() - 5.0; // already expired
    db.record_compression_failure_cooldown("s1", until, Some("old failure"));

    // Active-cooldown API filters by expiry.
    assert_eq!(db.get_compression_failure_cooldown("s1").expect("get"), None);
    // Raw-row API preserves the exact expired row (rollback contract).
    let row = db.get_compression_failure_cooldown_row("s1").expect("row");
    let o = row.as_object().unwrap();
    assert_eq!(o["session_exists"], json!(true));
    assert_eq!(o["cooldown_until"].as_f64(), Some(until));
    assert_eq!(o["error"].as_str(), Some("old failure"));
}

#[test]
fn cooldown_row_absent_and_missing_session() {
    let (_dir, db) = open_db("state.db");
    // Missing session -> session_exists=false.
    let row = db.get_compression_failure_cooldown_row("nope").expect("row");
    assert_eq!(row, json!({"session_exists": false, "cooldown_until": Value::Null, "error": Value::Null}));
    assert_eq!(db.get_compression_failure_cooldown("nope").expect("get"), None);
    // Existing session without a stored cooldown -> exists, NULL columns.
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let row = db.get_compression_failure_cooldown_row("s1").expect("row2");
    let o = row.as_object().unwrap();
    assert_eq!(o["session_exists"], json!(true));
    assert_eq!(o["cooldown_until"], Value::Null);
    assert_eq!(o["error"], Value::Null);
}

#[test]
fn restore_roundtrip_and_absent_session_guard() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let deadline = now() + 30.0;
    let snapshot = json!({
        "session_exists": true,
        "cooldown_until": deadline,
        "error": "restored error",
    });
    db.restore_compression_failure_cooldown_row("s1", &snapshot).expect("restore");
    let actual = db.get_compression_failure_cooldown_row("s1").expect("row");
    assert_eq!(actual, snapshot);

    // NULL deadline restores a partially-null row exactly.
    db.restore_compression_failure_cooldown_row(
        "s1",
        &json!({"session_exists": true, "cooldown_until": Value::Null, "error": Value::Null}),
    )
    .expect("restore null");
    let actual = db.get_compression_failure_cooldown_row("s1").expect("row2");
    assert_eq!(actual["session_exists"], json!(true));
    assert_eq!(actual["cooldown_until"], Value::Null);
    assert_eq!(actual["error"], Value::Null);

    // Absent-snapshot guard: restoring "absent" for an existing session is an
    // error, and the row must remain untouched.
    let err = db
        .restore_compression_failure_cooldown_row("s1", &json!({"session_exists": false}))
        .expect_err("should fail");
    assert!(matches!(err, WriteError::Runtime(_)));
    let actual = db.get_compression_failure_cooldown_row("s1").expect("row3");
    assert_eq!(actual["session_exists"], json!(true));

    // Absent-snapshot for a truly absent session is a no-op success.
    db.restore_compression_failure_cooldown_row("nope", &json!({"session_exists": false}))
        .expect("absent no-op");
    // Missing-session restore with session_exists=true fails loudly.
    let err = db
        .restore_compression_failure_cooldown_row(
            "nope",
            &json!({"session_exists": true, "cooldown_until": now(), "error": "x"}),
        )
        .expect_err("missing session");
    assert!(matches!(err, WriteError::Runtime(_)));
}

#[test]
fn restore_verification_catches_divergence() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let snapshot = json!({
        "session_exists": true,
        "cooldown_until": now() + 10.0,
        "error": "E",
    });
    db.restore_compression_failure_cooldown_row("s1", &snapshot).expect("restore");
    let actual = db.get_compression_failure_cooldown_row("s1").expect("row");
    assert_eq!(actual, snapshot);
}

#[test]
fn clear_cooldown_and_empty_id_noops() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.record_compression_failure_cooldown("s1", now() + 100.0, Some("x"));
    db.clear_compression_failure_cooldown("s1");
    let (raw_until, raw_err) = cooldown_cols(&db, "s1");
    assert_eq!(raw_until, None);
    assert_eq!(raw_err, None);
    assert_eq!(db.get_compression_failure_cooldown("s1").expect("get"), None);

    // Empty session ids are silent no-ops on every API.
    db.record_compression_failure_cooldown("", 1.0, None);
    db.clear_compression_failure_cooldown("");
    assert_eq!(db.get_compression_failure_cooldown("").expect("empty"), None);
    assert_eq!(db.get_compression_failure_cooldown_row("").expect("empty row"),
        json!({"session_exists": false, "cooldown_until": Value::Null, "error": Value::Null}));
    assert_eq!(db.get_compression_fallback_streak("").expect("empty streak"), 0);
    db.set_compression_fallback_streak("", 5).expect("set empty");
    assert_eq!(db.get_compression_ineffective_count("").expect("empty count"), 0);
    db.set_compression_ineffective_count("", 5).expect("set empty");
}

#[test]
fn fallback_streak_roundtrip_and_clamp() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    assert_eq!(db.get_compression_fallback_streak("s1").expect("default"), 0);
    // Missing session reads as 0.
    assert_eq!(db.get_compression_fallback_streak("nope").expect("missing"), 0);

    db.set_compression_fallback_streak("s1", 3).expect("set 3");
    assert_eq!(db.get_compression_fallback_streak("s1").expect("get 3"), 3);
    db.set_compression_fallback_streak("s1", 0).expect("set 0");
    assert_eq!(db.get_compression_fallback_streak("s1").expect("get 0"), 0);
    // Negative normalization (upstream max(0, int(streak))).
    db.set_compression_fallback_streak("s1", -2).expect("set neg");
    assert_eq!(db.get_compression_fallback_streak("s1").expect("get clamped"), 0);
}

#[test]
fn ineffective_count_roundtrip_and_clamp() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    assert_eq!(db.get_compression_ineffective_count("s1").expect("default"), 0);

    db.set_compression_ineffective_count("s1", 4).expect("set 4");
    assert_eq!(db.get_compression_ineffective_count("s1").expect("get 4"), 4);
    db.set_compression_ineffective_count("s1", -1).expect("set neg");
    assert_eq!(db.get_compression_ineffective_count("s1").expect("get clamped"), 0);
}
