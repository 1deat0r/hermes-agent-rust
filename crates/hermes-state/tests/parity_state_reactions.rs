//! Parity oracles for message presentation + reaction surfaces, mirroring
//! upstream tests/test_message_reactions.py (tapback semantics, take-unseen
//! exactly-once, author independence, cache safety) @ b9aa928.

use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::state::SessionDB;
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

fn msg(role: &str, content: &str) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(serde_json::json!(content)),
        ..Default::default()
    }
}

fn msg_row_ids(db: &SessionDB, sid: &str) -> Vec<i64> {
    let conn = db.writer_conn();
    let mut stmt = conn
        .prepare("SELECT id FROM messages WHERE session_id = ? ORDER BY id")
        .expect("stmt");
    let rows: Vec<i64> = stmt
        .query_map([sid], |r| r.get(0))
        .expect("map")
        .collect::<Result<_, _>>()
        .expect("collect");
    rows
}

fn session_scaffold(db: &SessionDB) -> (String, Vec<i64>) {
    let key = db.create_session("react-test", "test", &NewSession::default()).expect("create");
    db.append_message(&key, &msg("user", "how do i center a div"), None).expect("msg1");
    db.append_message(&key, &msg("assistant", "use flexbox"), None).expect("msg2");
    let rows = msg_row_ids(db, &key);
    (key, rows)
}

fn reactions_of(db: &SessionDB, key: &str, row: i64) -> Vec<Value> {
    db.get_message_reactions(key, row)
        .expect("reactions")
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn row_ids_follow_insertion_order() {
    let (_dir, db) = open_db("state.db");
    let (_key, rows) = session_scaffold(&db);
    assert_eq!(rows.len(), 2);
    assert!(rows[0] < rows[1]);
}

#[test]
fn one_reaction_per_author_replaces() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);

    db.set_message_reaction(&key, rows[0], Some("\u{2764}\u{fe0f}"), "user").expect("r1");
    let reactions = db
        .set_message_reaction(&key, rows[0], Some("\u{1f602}"), "user")
        .expect("r2")
        .expect("some");
    let emojis: Vec<&str> = reactions
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r.get("emoji").and_then(Value::as_str))
        .collect();
    assert_eq!(emojis, vec!["\u{1f602}"]);
}

#[test]
fn repeating_an_emoji_retracts_it() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);

    db.set_message_reaction(&key, rows[0], Some("\u{1f44d}"), "user").expect("r1");
    let reactions = db
        .set_message_reaction(&key, rows[0], Some("\u{1f44d}"), "user")
        .expect("r2")
        .expect("some");
    assert_eq!(reactions, json!([]));
    assert!(reactions_of(&db, &key, rows[0]).is_empty());
}

#[test]
fn authors_are_independent() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);

    db.set_message_reaction(&key, rows[0], Some("\u{2764}\u{fe0f}"), "user").expect("user");
    let reactions = db
        .set_message_reaction(&key, rows[0], Some("\u{1f525}"), "agent")
        .expect("agent")
        .expect("some");
    let authors: Vec<&str> = reactions
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r.get("author").and_then(Value::as_str))
        .collect();
    let mut sorted = authors.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec!["agent", "user"]);

    let remaining = db
        .set_message_reaction(&key, rows[0], None, "user")
        .expect("clear")
        .expect("some");
    let authors: Vec<&str> = remaining
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r.get("author").and_then(Value::as_str))
        .collect();
    assert_eq!(authors, vec!["agent"]);
}

#[test]
fn rejects_rows_outside_the_session() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);

    assert_eq!(
        db.set_message_reaction(&key, 9999, Some("\u{2764}\u{fe0f}"), "user").expect("bad row"),
        None
    );
    assert_eq!(
        db.set_message_reaction("no-such-session", rows[0], Some("\u{2764}\u{fe0f}"), "user")
            .expect("bad session"),
        None
    );
}

#[test]
fn clearing_every_reaction_removes_metadata_key() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);

    db.set_message_reaction(&key, rows[0], Some("\u{2764}\u{fe0f}"), "user").expect("r1");
    db.set_message_reaction(&key, rows[0], None, "user").expect("clear");

    let conn = db.writer_conn();
    let meta: Option<String> = conn
        .query_row(
            "SELECT display_metadata FROM messages WHERE id = ?",
            rusqlite::params![rows[0]],
            |r| r.get(0),
        )
        .expect("meta");
    assert_eq!(meta, None);
}

#[test]
fn reactions_survive_reload() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);
    db.set_message_reaction(&key, rows[1], Some("\u{1f525}"), "agent").expect("r1");

    let path = _dir.path().join("state.db");
    drop(db);
    // Reopen the same file through a fresh SessionDB.
    let reopened = SessionDB::open(Some(path), false).expect("reopen");
    let reactions = reopened.get_message_reactions(&key, rows[1]).expect("get");
    let emojis: Vec<&str> = reactions
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r.get("emoji").and_then(Value::as_str))
        .collect();
    assert_eq!(emojis, vec!["\u{1f525}"]);
}

#[test]
fn unseen_reactions_are_taken_exactly_once() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);
    db.set_message_reaction(&key, rows[1], Some("\u{2764}\u{fe0f}"), "user").expect("r1");

    let taken = db.take_unseen_reactions(&key, "user").expect("first");
    let first = taken.as_array().unwrap();
    assert_eq!(first[0]["emoji"].as_str(), Some("\u{2764}\u{fe0f}"));
    assert_eq!(first[0]["row_id"].as_i64(), Some(rows[1]));
    assert_eq!(first[0]["text"].as_str(), Some("use flexbox"));

    assert_eq!(
        db.take_unseen_reactions(&key, "user").expect("second"),
        json!([])
    );
}

#[test]
fn new_reaction_becomes_unseen_again() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);
    db.set_message_reaction(&key, rows[1], Some("\u{2764}\u{fe0f}"), "user").expect("r1");
    db.take_unseen_reactions(&key, "user").expect("take");

    db.set_message_reaction(&key, rows[1], Some("\u{1f525}"), "user").expect("r2");
    let taken = db.take_unseen_reactions(&key, "user").expect("take2");
    let emojis: Vec<&str> = taken
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r.get("emoji").and_then(Value::as_str))
        .collect();
    assert_eq!(emojis, vec!["\u{1f525}"]);
}

#[test]
fn take_unseen_filters_by_author() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);
    db.set_message_reaction(&key, rows[0], Some("\u{1f60a}"), "agent").expect("r1");
    assert_eq!(
        db.take_unseen_reactions(&key, "user").expect("take"),
        json!([])
    );
}

#[test]
fn set_latest_matching_message_display_kind_stamps_turn() {
    let (_dir, db) = open_db("state.db");
    let (key, _rows) = session_scaffold(&db);
    // The latest user message is "how do i center a div".
    let ok = db
        .set_latest_matching_message_display_kind(
            &key,
            "user",
            Some(&json!("how do i center a div")),
            "command",
            Some(&json!({"source": "cli"})),
        )
        .expect("stamp");
    assert!(ok);
    let conn = db.writer_conn();
    let (kind, meta): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT display_kind, display_metadata FROM messages \
             WHERE session_id = ? AND role = 'user' ORDER BY id DESC LIMIT 1",
            rusqlite::params![key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("row");
    assert_eq!(kind.as_deref(), Some("command"));
    assert!(meta.as_deref().unwrap_or("").contains("cli"));

    // Non-matching content returns false and writes nothing.
    let ok = db
        .set_latest_matching_message_display_kind(
            &key,
            "user",
            Some(&json!("nope")),
            "command",
            None,
        )
        .expect("stamp nomatch");
    assert!(!ok);
}

#[test]
fn set_latest_user_api_content_backfills_sidecar() {
    let (_dir, db) = open_db("state.db");
    let (key, rows) = session_scaffold(&db);
    let stamped = db
        .set_latest_user_api_content(&key, Some(&json!("how do i center a div")), "STAMPED-API")
        .expect("backfill");
    assert_eq!(stamped, 1);
    let conn = db.writer_conn();
    let api: Option<String> = conn
        .query_row(
            "SELECT api_content FROM messages WHERE id = ?",
            rusqlite::params![rows[0]],
            |r| r.get(0),
        )
        .expect("api");
    assert_eq!(api.as_deref(), Some("STAMPED-API"));

    // Defensive content guard: non-matching newest user row writes nothing.
    let stamped = db
        .set_latest_user_api_content(&key, Some(&json!("other content")), "NOPE")
        .expect("guard");
    assert_eq!(stamped, 0);
}
