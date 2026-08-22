//! Parity oracles for the gateway routing surface, mirroring upstream
//! tests/test_hermes_state.py (test_gateway_session_peer_round_trip_and_
//! recovery, test_find_session_by_origin_matching_rules) plus the routing
//! entry CRUD roundtrip tested by gateway SessionStore tests @ b9aa928.

use std::collections::HashMap;
use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::state::SessionDB;

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

fn msg(role: &str, content: &str) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(serde_json::json!(content)),
        ..Default::default()
    }
}

fn col_str(db: &SessionDB, col: &str, sid: &str) -> Option<String> {
    let conn = db.writer_conn();
    conn.query_row(
        &format!("SELECT {col} FROM sessions WHERE id = ?"),
        rusqlite::params![sid],
        |r| r.get(0),
    )
    .ok()
}

fn col_int(db: &SessionDB, col: &str, sid: &str) -> Option<i64> {
    let conn = db.writer_conn();
    conn.query_row(
        &format!("SELECT {col} FROM sessions WHERE id = ?"),
        rusqlite::params![sid],
        |r| r.get(0),
    )
    .ok()
}

#[test]
fn gateway_session_peer_round_trip_and_recovery() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "gw-session",
        "telegram",
        &NewSession {
            user_id: Some("user-1".into()),
            session_key: Some("agent:main:telegram:dm:chat-1".into()),
            chat_id: Some("chat-1".into()),
            chat_type: Some("dm".into()),
            thread_id: None,
            ..Default::default()
        },
    )
    .expect("create");
    db.append_message("gw-session", &msg("user", "hello"), None)
        .expect("append");

    let row = db.get_session("gw-session").expect("get").expect("row");
    assert_eq!(row.session_key.as_deref(), Some("agent:main:telegram:dm:chat-1"));
    assert_eq!(row.chat_id.as_deref(), Some("chat-1"));
    assert_eq!(row.chat_type.as_deref(), Some("dm"));

    let recovered = db
        .find_latest_gateway_session_for_peer(
            "telegram",
            Some("user-1"),
            Some("agent:main:telegram:dm:chat-1"),
            Some("chat-1"),
            Some("dm"),
            None,
        )
        .expect("recover")
        .expect("some");
    assert_eq!(recovered["id"].as_str(), Some("gw-session"));
}

#[test]
fn find_session_by_origin_matching_rules() {
    let (_dir, db) = open_db("state.db");
    let group = |_sid: &str, uid: &str, key: &str| NewSession {
        user_id: Some(uid.into()),
        session_key: Some(key.into()),
        chat_id: Some("c9".into()),
        chat_type: Some("group".into()),
        ..Default::default()
    };
    db.create_session("gw-o1", "telegram", &group("gw-o1", "u1", "agent:main:telegram:group:c9:u1"))
        .expect("o1");
    db.create_session("gw-o2", "telegram", &group("gw-o2", "u2", "agent:main:telegram:group:c9:u2"))
        .expect("o2");

    // Exact user match wins.
    assert_eq!(
        db.find_session_by_origin("telegram", "c9", None, Some("u2")).expect("u2"),
        Some("gw-o2".into())
    );
    // Unknown user among multiple distinct users -> None (no contamination).
    assert_eq!(
        db.find_session_by_origin("telegram", "c9", None, Some("u3")).expect("u3"),
        None
    );
    // No user given + multiple distinct users -> None.
    assert_eq!(db.find_session_by_origin("telegram", "c9", None, None).expect("multi"), None);
    // Ended sessions are ignored: only gw-o1 remains as a live candidate.
    db.end_session("gw-o2", "session_reset").expect("end o2");
    assert_eq!(
        db.find_session_by_origin("telegram", "c9", None, Some("u2")).expect("u2b"),
        Some("gw-o1".into())
    );
    assert_eq!(
        db.find_session_by_origin("telegram", "c9", None, None).expect("single"),
        Some("gw-o1".into())
    );
    // Thread filter.
    db.create_session(
        "gw-th",
        "discord",
        &NewSession {
            user_id: Some("u9".into()),
            session_key: Some("agent:main:discord:thread:t7".into()),
            chat_id: Some("ch7".into()),
            chat_type: Some("thread".into()),
            thread_id: Some("t7".into()),
            ..Default::default()
        },
    )
    .expect("th");
    assert_eq!(
        db.find_session_by_origin("discord", "ch7", Some("t7"), None).expect("t7"),
        Some("gw-th".into())
    );
    assert_eq!(
        db.find_session_by_origin("discord", "ch7", Some("other"), None).expect("other"),
        None
    );
    // Missing chat/empty platform short-circuit.
    assert_eq!(db.find_session_by_origin("telegram", "", None, None).expect("empty chat"), None);
    assert_eq!(db.find_session_by_origin("", "c9", None, None).expect("empty plat"), None);
}

#[test]
fn record_gateway_session_peer_sets_and_coalesces() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");

    db.record_gateway_session_peer(
        "s1",
        "telegram",
        Some("user-1"),
        Some("agent:main:telegram:dm:lane"),
        Some("lane"),
        Some("dm"),
        None,
        Some("Display Name"),
        None,
        false,
    )
    .expect("record");
    assert_eq!(col_str(&db, "session_key", "s1").as_deref(), Some("agent:main:telegram:dm:lane"));
    assert_eq!(col_str(&db, "source", "s1").as_deref(), Some("telegram"));
    assert_eq!(col_str(&db, "display_name", "s1").as_deref(), Some("Display Name"));

    // COALESCE: None leaves the existing display_name / origin_json untouched.
    db.record_gateway_session_peer(
        "s1",
        "telegram",
        Some("user-1"),
        Some("agent:main:telegram:dm:lane"),
        Some("lane"),
        Some("dm"),
        None,
        None,
        Some(r#"{"chat_title":"volleyball"}"#),
        false,
    )
    .expect("record2");
    assert_eq!(col_str(&db, "display_name", "s1").as_deref(), Some("Display Name"));
    assert_eq!(col_str(&db, "origin_json", "s1").as_deref(), Some(r#"{"chat_title":"volleyball"}"#));

    // Empty session_key short-circuits.
    db.record_gateway_session_peer(
        "s1",
        "x", None, Some(""), None, None, None, None, None, false,
    )
    .expect("empty key no-op");
    assert_eq!(col_str(&db, "source", "s1").as_deref(), Some("telegram"));
}

#[test]
fn record_gateway_session_peer_stamps_compression_lineage() {
    let (_dir, db) = open_db("state.db");
    let mut t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    db.create_session("root", "cli", &NewSession::default()).expect("root");
    db.append_message("root", &msg("user", "hi"), None).expect("append");
    t += 100.0;
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET ended_at = ?, end_reason = 'compression' WHERE id = 'root'",
        rusqlite::params![t],
    )
    .expect("end root");
    db.create_session(
        "tip",
        "telegram",
        &NewSession {
            parent_session_id: Some("root".into()),
            ..Default::default()
        },
    )
    .expect("tip");

    // Peer metadata on tip propagates to the whole compression lineage.
    db.record_gateway_session_peer(
        "tip",
        "telegram",
        Some("lane-user"),
        Some("agent:main:telegram:dm:lane"),
        Some("lane"),
        Some("dm"),
        None,
        Some("Tip Name"),
        None,
        true,
    )
    .expect("record lineage");
    assert_eq!(col_str(&db, "session_key", "tip").as_deref(), Some("agent:main:telegram:dm:lane"));
    assert_eq!(col_str(&db, "session_key", "root").as_deref(), Some("agent:main:telegram:dm:lane"));
    assert_eq!(col_str(&db, "display_name", "root").as_deref(), Some("Tip Name"));
}

#[test]
fn routing_entries_upsert_replace_load_delete() {
    let (_dir, db) = open_db("state.db");
    let scope_a = "/tmp/state/one";
    let scope_b = "/tmp/state/two";

    db.save_gateway_routing_entry("k1", r#"{"kind":"dm"}"#, scope_a).expect("save k1");
    db.save_gateway_routing_entry("k2", r#"{"kind":"group"}"#, scope_a).expect("save k2");
    db.save_gateway_routing_entry("other", "{}", scope_b).expect("save other");

    let loaded = db.load_gateway_routing_entries(scope_a).expect("load a");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get("k1").map(|s| s.as_str()), Some(r#"{"kind":"dm"}"#));
    // Scopes are namespaced.
    assert_eq!(db.load_gateway_routing_entries(scope_b).expect("load b").len(), 1);

    // Upsert overwrites entry_json + bumps updated_at.
    let before = db.load_gateway_routing_entries(scope_a).expect("before");
    let before_upd = {
        let conn = db.writer_conn();
        conn.query_row(
            "SELECT updated_at FROM gateway_routing WHERE scope = ? AND session_key = ?",
            rusqlite::params![scope_a, "k1"],
            |r| r.get::<_, f64>(0),
        )
        .expect("upd")
    };
    db.save_gateway_routing_entry("k1", r#"{"kind":"dm","pinned":true}"#, scope_a)
        .expect("upsert k1");
    let after = db.load_gateway_routing_entries(scope_a).expect("after");
    assert_eq!(after.get("k1").map(|s| s.as_str()), Some(r#"{"kind":"dm","pinned":true}"#));
    let after_upd = {
        let conn = db.writer_conn();
        conn.query_row(
            "SELECT updated_at FROM gateway_routing WHERE scope = ? AND session_key = ?",
            rusqlite::params![scope_a, "k1"],
            |r| r.get::<_, f64>(0),
        )
        .expect("upd2")
    };
    assert!(after_upd >= before_upd);
    assert_eq!(before.len(), after.len());

    // Replace removes keys absent from the new map.
    let mut entries = HashMap::new();
    entries.insert("k1".to_string(), r#"{"only":true}"#.to_string());
    db.replace_gateway_routing_entries(&entries, scope_a).expect("replace");
    let loaded = db.load_gateway_routing_entries(scope_a).expect("reload");
    assert_eq!(loaded.len(), 1);
    assert!(loaded.contains_key("k1"));
    assert!(!loaded.contains_key("k2"));

    // Delete removes only the given keys.
    db.delete_gateway_routing_entries(&["k1".to_string()], scope_a).expect("delete");
    assert!(db.load_gateway_routing_entries(scope_a).expect("empty").is_empty());
    // Other scope untouched.
    assert_eq!(db.load_gateway_routing_entries(scope_b).expect("b intact").len(), 1);
}

#[test]
fn set_expiry_finalized_roundtrip() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    assert_eq!(col_int(&db, "expiry_finalized", "s1"), Some(0));

    db.set_expiry_finalized("s1", true).expect("finalize");
    assert_eq!(col_int(&db, "expiry_finalized", "s1"), Some(1));

    db.set_expiry_finalized("s1", false).expect("unfinalize");
    assert_eq!(col_int(&db, "expiry_finalized", "s1"), Some(0));

    // Empty session id is a silent no-op.
    db.set_expiry_finalized("", true).expect("no-op");
}
