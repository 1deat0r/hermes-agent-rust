//! Parity oracles for the Telegram DM topic-mode surface, mirroring
//! upstream tests/test_hermes_state.py
//! (test_telegram_topic_binding_roundtrip_requires_explicit_schema) and
//! tests/gateway/test_telegram_prune_stale_topic_binding_31501.py
//! (TestDeleteTelegramTopicBinding, TestPruneClearsTopicModeWhenLastBindingGone)
//! @ b9aa928. The gateway/adapter glue tests (source-level wiring guards,
//! _prune_stale_dm_topic_binding) land with the P2 gateway crate.

use std::path::PathBuf;

use hermes_state::crud::NewSession;
use hermes_state::state::{SessionDB, WriteError};
use serde_json::Value;

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

fn seed_binding(db: &SessionDB, chat_id: &str, thread_id: &str, user_id: &str, session_id: &str) {
    db.create_session(
        session_id,
        "telegram",
        &NewSession {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.bind_telegram_topic(
        chat_id,
        thread_id,
        user_id,
        &format!("agent:main:telegram:dm:{chat_id}:{thread_id}"),
        session_id,
        "auto",
    )
    .expect("bind");
}

#[test]
fn topic_binding_roundtrip_requires_explicit_schema() {
    // Mirrors test_hermes_state.py roundtrip test: reads before any
    // migration return None; bind creates the tables; the schema-version
    // marker lands at "2"; the binding round-trips.
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "topic-session",
        "telegram",
        &NewSession {
            user_id: Some("208214988".to_string()),
            ..Default::default()
        },
    )
    .expect("create");

    assert!(db
        .get_telegram_topic_binding("208214988", "17585")
        .expect("get")
        .is_none());
    // No tables before explicit opt-in.
    assert_eq!(db.get_meta("telegram_dm_topic_schema_version"), None);

    db.bind_telegram_topic(
        "208214988",
        "17585",
        "208214988",
        "telegram:dm:208214988:thread:17585",
        "topic-session",
        "auto",
    )
    .expect("bind");

    let binding = db
        .get_telegram_topic_binding("208214988", "17585")
        .expect("get")
        .expect("binding");
    assert_eq!(binding.chat_id, "208214988");
    assert_eq!(binding.thread_id, "17585");
    assert_eq!(binding.user_id, "208214988");
    assert_eq!(binding.session_key, "telegram:dm:208214988:thread:17585");
    assert_eq!(binding.session_id, "topic-session");
    assert_eq!(binding.managed_mode, "auto");
    assert_eq!(
        db.get_meta("telegram_dm_topic_schema_version").as_deref(),
        Some("2")
    );
    db.close();
}

#[test]
fn bind_same_topic_idempotent_other_topic_raises_value_error() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "sess-linked",
        "telegram",
        &NewSession {
            user_id: Some("1".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.bind_telegram_topic("c1", "t1", "1", "k1", "sess-linked", "auto")
        .expect("bind");
    // Same topic → idempotent upsert.
    db.bind_telegram_topic("c1", "t1", "1", "k1", "sess-linked", "auto")
        .expect("bind");
    // Different topic → ValueError.
    let err = db
        .bind_telegram_topic("c1", "t2", "1", "k2", "sess-linked", "auto")
        .unwrap_err();
    match err {
        WriteError::ValueError(msg) => {
            assert!(msg.contains("already linked"), "msg: {msg}")
        }
        other => panic!("expected ValueError, got {other:?}"),
    }
    db.close();
}

#[test]
fn reverse_lookup_and_linked_probe() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "sess-r",
        "telegram",
        &NewSession {
            user_id: Some("7".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    // No tables yet → not linked.
    assert!(!db.is_telegram_session_linked_to_topic("sess-r"));
    assert!(db
        .get_telegram_topic_binding_by_session("sess-r")
        .expect("get")
        .is_none());

    db.bind_telegram_topic("c7", "t9", "7", "k7", "sess-r", "manual")
        .expect("bind");
    assert!(db.is_telegram_session_linked_to_topic("sess-r"));
    let by_session = db
        .get_telegram_topic_binding_by_session("sess-r")
        .expect("get")
        .expect("binding");
    assert_eq!(by_session.chat_id, "c7");
    assert_eq!(by_session.thread_id, "t9");
    assert_eq!(by_session.managed_mode, "manual");
    db.close();
}

#[test]
fn enable_and_disable_topic_mode() {
    let (_dir, db) = open_db("state.db");
    // Read before migration → false.
    assert!(!db.is_telegram_topic_mode_enabled("67890", "12345"));

    db.enable_telegram_topic_mode("67890", "12345", Some(true), Some(false))
        .expect("enable");
    assert!(db.is_telegram_topic_mode_enabled("67890", "12345"));
    // Different user not enabled.
    assert!(!db.is_telegram_topic_mode_enabled("67890", "nobody"));
    // Capability flags round-trip into the mode row.
    let v = db
        .get_telegram_topic_bound_cols("67890", "12345")
        .expect("mode cols");
    assert_eq!(v, Some((Some(1), Some(0))));

    db.disable_telegram_topic_mode("67890", true)
        .expect("disable");
    assert!(!db.is_telegram_topic_mode_enabled("67890", "12345"));
    // Disable when tables are absent is a silent no-op (fresh DB).
    let (_dir2, db2) = open_db("state.db");
    db2.disable_telegram_topic_mode("noop-chat", true)
        .expect("noop");
    db2.close();
    db.close();
}

#[test]
fn list_bindings_for_chat_newest_first() {
    let (_dir, db) = open_db("state.db");
    for (i, tid) in ["t1", "t2"].iter().enumerate() {
        let sid = format!("s-{}", i);
        db.create_session(
            &sid,
            "telegram",
            &NewSession {
                user_id: Some("u1".to_string()),
                ..Default::default()
            },
        )
        .expect("create");
        db.bind_telegram_topic("chat-1", tid, "u1", &format!("k{}", i), &sid, "auto")
            .expect("bind");
    }
    let list = db
        .list_telegram_topic_bindings_for_chat("chat-1")
        .expect("list");
    // Insert order t1 → t2; same-second timestamps can tie, so assert the
    // set and the newest-first ordering only when timestamps differ.
    let ids: Vec<String> = list.iter().map(|b| b.thread_id.clone()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"t1".to_string()) && ids.contains(&"t2".to_string()));
    assert!(ids[0] == "t2" || list[0].updated_at >= list[1].updated_at);
    // Absent-table list is empty.
    let (_dir2, db2) = open_db("state.db");
    assert_eq!(
        db2.list_telegram_topic_bindings_for_chat("chat-1")
            .expect("list")
            .len(),
        0
    );
    db2.close();
    db.close();
}

#[test]
fn delete_prunes_only_target_binding() {
    let (_dir, db) = open_db("state.db");
    seed_binding(&db, "5595856929", "15287", "5595856929", "sess-stale");
    seed_binding(&db, "5595856929", "15418", "5595856929", "sess-fresh");

    let removed = db
        .delete_telegram_topic_binding("5595856929", "15287")
        .expect("delete");
    assert_eq!(removed, 1);
    assert!(db
        .get_telegram_topic_binding("5595856929", "15287")
        .expect("get")
        .is_none());
    assert!(db
        .get_telegram_topic_binding("5595856929", "15418")
        .expect("get")
        .is_some());
    // Deleting an absent pair → 0 (silent).
    let removed = db
        .delete_telegram_topic_binding("5595856929", "nope")
        .expect("delete");
    assert_eq!(removed, 0);
    db.close();
}

#[test]
fn prune_clears_topic_mode_when_last_binding_gone() {
    let (_dir, db) = open_db("state.db");
    db.enable_telegram_topic_mode("5595856929", "5595856929", None, None)
        .expect("enable");
    seed_binding(&db, "5595856929", "15287", "5595856929", "sess-target");
    assert!(db.is_telegram_topic_mode_enabled("5595856929", "5595856929"));

    let removed = db
        .delete_telegram_topic_binding("5595856929", "15287")
        .expect("delete");
    assert_eq!(removed, 1);
    assert!(!db.is_telegram_topic_mode_enabled("5595856929", "5595856929"));
    db.close();
}

#[test]
fn migration_rebuilds_v1_fk_to_cascade() {
    // The v1 → v2 gate rebuilds telegram_dm_topic_bindings when its
    // session_id FK lacks ON DELETE CASCADE (version < 2).
    let (_dir, db) = open_db("state.db");
    db.set_meta("telegram_dm_topic_schema_version", "1")
        .expect("meta");
    {
        let conn = db.writer_conn();
        conn.execute_batch(
            "CREATE TABLE telegram_dm_topic_bindings (
                chat_id TEXT NOT NULL, thread_id TEXT NOT NULL, user_id TEXT NOT NULL,
                session_key TEXT NOT NULL, session_id TEXT NOT NULL REFERENCES sessions(id),
                managed_mode TEXT NOT NULL DEFAULT 'auto', linked_at REAL NOT NULL,
                updated_at REAL NOT NULL, PRIMARY KEY (chat_id, thread_id)
            );",
        )
        .expect("v1 table");
    }
    db.apply_telegram_topic_migration().expect("migrate");
    assert_eq!(
        db.get_meta("telegram_dm_topic_schema_version").as_deref(),
        Some("2")
    );
    let conn = db.writer_conn();
    let mut stmt = conn
        .prepare("PRAGMA foreign_key_list('telegram_dm_topic_bindings')")
        .expect("fk stmt");
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(2)?, r.get(6)?)))
        .expect("fk rows")
        .map(|r| r.unwrap())
        .collect();
    drop(stmt);
    drop(conn);
    assert!(
        rows.iter().any(|(t, d)| t == "sessions" && d == "CASCADE"),
        "rebuild must add ON DELETE CASCADE: rows={rows:?}"
    );
    db.close();
}

#[test]
fn unlinked_sessions_fallback_when_tables_absent() {
    // Mirrors the absent-tables branch of
    // list_unlinked_telegram_sessions_for_user: without the topic tables
    // every telegram session is "unlinked", with previews shaped.
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "sess-u1",
        "telegram",
        &NewSession {
            user_id: Some("u1".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.create_session(
        "sess-u2",
        "telegram",
        &NewSession {
            user_id: Some("u2".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.create_session(
        "sess-cli",
        "cli",
        &NewSession {
            user_id: Some("u1".to_string()),
            ..Default::default()
        },
    )
    .expect("create");

    let rows = db
        .list_unlinked_telegram_sessions_for_user("chat-1", "u1", 10)
        .expect("list");
    // Only u1's telegram sessions; cli + other-user excluded.
    let ids: Vec<&str> = rows.iter().map(|v| v["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], "sess-u1");
    // Preview key present (=== _shape_preview of '' when no messages).
    assert_eq!(rows[0].get("preview").and_then(Value::as_str), Some(""));
    db.close();
}

#[test]
fn unlinked_sessions_excludes_topic_bound() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "sess-bound",
        "telegram",
        &NewSession {
            user_id: Some("u1".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.create_session(
        "sess-free",
        "telegram",
        &NewSession {
            user_id: Some("u1".to_string()),
            ..Default::default()
        },
    )
    .expect("create");
    db.bind_telegram_topic("chat-1", "t1", "u1", "k1", "sess-bound", "auto")
        .expect("bind");

    let rows = db
        .list_unlinked_telegram_sessions_for_user("chat-1", "u1", 10)
        .expect("list");
    let ids: Vec<&str> = rows.iter().map(|v| v["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["sess-free"]);
    db.close();
}
