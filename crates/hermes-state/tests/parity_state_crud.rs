//! Parity oracles for the SessionDB CRUD surface — sessions + messages +
//! titles, mirroring upstream tests/test_hermes_state.py
//! (TestSessionLifecycle, TestMessageStorage, TestTimestampPreservation,
//! TestSessionTitle, TestSanitizeTitle, TestTitleLineage,
//! TestTitleSqlWildcards, TestSessionTitleLineage) and
//! tests/hermes_state/test_append_messages_batch.py (@ b9aa928).

use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::state::SessionDB;
use rusqlite::Connection;
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

fn msg(role: &str, content: impl Into<Value>) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(content.into()),
        ..Default::default()
    }
}

// =====================================================================
// TestSessionLifecycle
// =====================================================================

#[test]
fn create_and_get_session() {
    let (_dir, db) = open_db("state.db");
    let sid = db
        .create_session(
            "s1",
            "cli",
            &NewSession {
                model: Some("test-model".to_string()),
                ..Default::default()
            },
        )
        .expect("create");
    assert_eq!(sid, "s1");

    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.source, "cli");
    assert_eq!(session.model.as_deref(), Some("test-model"));
    assert!(session.ended_at.is_none());
    db.close();
}

#[test]
fn update_session_cwd_persists_git_branch() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.update_session_cwd("s1", "/work/repo", Some("pets-feature"), None, false)
        .expect("cwd");

    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.cwd.as_deref(), Some("/work/repo"));
    assert_eq!(session.git_branch.as_deref(), Some("pets-feature"));
    db.close();
}

#[test]
fn end_session_first_reason_wins_across_concurrent_connections() {
    // Mirrors TestSessionLifecycle.test_end_session_first_reason_wins...:
    // two finalizers race; the first end_reason sticks.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path.clone()), false).expect("open");
    db.create_session("s1", "cron", &NewSession::default()).expect("create");
    {
        let conn = db.writer_conn();
        conn.execute_batch(
            "CREATE TABLE session_end_audit (reason TEXT NOT NULL);
             CREATE TRIGGER audit_session_end
             AFTER UPDATE OF ended_at ON sessions
             WHEN OLD.ended_at IS NULL AND NEW.ended_at IS NOT NULL
             BEGIN
                 INSERT INTO session_end_audit(reason) VALUES (NEW.end_reason);
             END",
        )
        .unwrap();
    }
    let peer = SessionDB::open(Some(path.clone()), false).expect("peer open");

    // Each thread owns its own SessionDB instance (upstream: `db` and `peer`
    // are separate connections; writer connection is thread-affine here).
    let (a, b) = (db, peer);
    let t1 = std::thread::spawn(move || {
        a.end_session("s1", "compression").expect("end1");
        a.close();
    });
    let t2 = std::thread::spawn(move || {
        b.end_session("s1", "cron_complete").expect("end2");
        b.close();
    });
    t1.join().unwrap();
    t2.join().unwrap();

    let audit: Vec<String> = Connection::open(&path)
        .unwrap()
        .prepare("SELECT reason FROM session_end_audit")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(audit.len(), 1);
    let winner_conn = Connection::open(&path).unwrap();
    let end_reason: Option<String> = winner_conn
        .query_row("SELECT end_reason FROM sessions WHERE id = 's1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(end_reason.as_deref(), Some(audit[0].as_str()));
}

// =====================================================================
// TestMessageStorage
// =====================================================================

#[test]
fn append_and_get_messages() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.append_message("s1", &msg("user", "Hello"), None).expect("append");
    db.append_message("s1", &msg("assistant", "Hi there!"), None).expect("append");

    let messages = db.get_messages("s1", false, None, 0).expect("get");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content.as_ref().unwrap(), &json!("Hello"));
    assert_eq!(messages[1].role, "assistant");
    db.close();
}

#[test]
fn append_message_returns_row_id_and_increments_counters() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let id = db.append_message("s1", &msg("user", "Hello"), None).expect("append");
    assert!(id > 0);
    let s = db.get_session("s1").expect("get").expect("row");
    assert_eq!(s.message_count, 1);
    assert_eq!(s.tool_call_count, 0);
    // Tool-call row increments tool_call_count.
    db.append_message(
        "s1",
        &MessageInput {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(json!([{"function": {"name": "cronjob", "arguments": "{}"}, "id": "c1", "type": "function"}])),
            ..Default::default()
        },
        None,
    )
    .expect("append tool");
    let s = db.get_session("s1").expect("get").expect("row");
    assert_eq!(s.message_count, 2);
    assert_eq!(s.tool_call_count, 1);
    db.close();
}

#[test]
fn reasoning_persisted_and_restored() {
    // TestMessageStorage.test_reasoning_persisted_and_restored — raw rows via
    // get_messages (get_messages_as_conversation lands with the read surface).
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "telegram", &NewSession::default()).expect("create");
    db.append_message("s1", &msg("user", "create a cron job"), None).expect("u");
    db.append_message(
        "s1",
        &MessageInput {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(json!([{"function": {"name": "cronjob", "arguments": "{}"}, "id": "c1", "type": "function"}])),
            reasoning: Some("I should call the cronjob tool to schedule this.".into()),
            ..Default::default()
        },
        None,
    )
    .expect("a");
    db.append_message(
        "s1",
        &MessageInput {
            role: "tool".into(),
            content: Some(json!("{\"job_id\": \"abc\"}")),
            tool_call_id: Some("c1".into()),
            ..Default::default()
        },
        None,
    )
    .expect("t");

    let rows = db.get_messages("s1", false, None, 0).expect("get");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].reasoning.as_deref(), Some("I should call the cronjob tool to schedule this."));
    assert!(rows[0].reasoning.is_none());
    assert!(rows[2].reasoning.is_none());
    db.close();
}

#[test]
fn multimodal_content_encoded_with_sentinel() {
    // append path JSON-encodes structured content; get_messages decodes it.
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let parts = json!([
        {"type": "text", "text": "look"},
        {"type": "image_url", "image_url": {"url": "data:x"}}
    ]);
    db.append_message(
        "s1",
        &MessageInput {
            role: "user".into(),
            content: Some(parts.clone()),
            ..Default::default()
        },
        None,
    )
    .expect("append");
    let rows = db.get_messages("s1", false, None, 0).expect("get");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].content.as_ref().unwrap(), &parts);
    db.close();
}

#[test]
fn explicit_timestamp_is_round_tripped() {
    // TestTimestampPreservation.test_append_message_with_explicit_timestamp
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    let ts = 1_234_567.0;
    let mid = db
        .append_message(
            "s1",
            &MessageInput { role: "user".into(), content: Some(json!("hello")), timestamp: Some(ts), ..Default::default() },
            None,
        )
        .expect("append");
    let rows = db.get_messages("s1", false, None, 0).expect("get");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].timestamp, ts);
    assert_eq!(rows[0].id, mid);
    let raw: f64 = db
        .writer_conn()
        .query_row(
            "SELECT timestamp FROM messages WHERE session_id = ? ORDER BY id",
            ["s1"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(raw, ts);
    db.close();
}

#[test]
fn latest_message_row_id_helpers() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    assert_eq!(db.latest_user_message_row_id("s1").unwrap(), None);
    assert_eq!(db.latest_message_row_id("s1", "user", 0, true).unwrap(), None);
    let u1 = db.append_message("s1", &msg("user", "one"), None).unwrap();
    let a1 = db.append_message("s1", &msg("assistant", "resp"), None).unwrap();
    let u2 = db.append_message("s1", &msg("user", "two"), None).unwrap();
    let a2 = db.append_message("s1", &msg("assistant", "resp2"), None).unwrap();
    assert_eq!(db.latest_message_row_id("s1", "user", 0, true).unwrap(), Some(u2));
    assert_eq!(db.latest_message_row_id("s1", "user", 1, true).unwrap(), Some(u1));
    assert_eq!(db.latest_message_row_id("s1", "assistant", 0, true).unwrap(), Some(a2));
    assert_eq!(db.latest_message_row_id("s1", "tool", 0, true).unwrap(), None);
    assert_eq!(db.latest_user_message_row_id("s1").unwrap(), Some(u2));
    assert_eq!(db.get_message_role("s1", a1).unwrap(), Some("assistant".to_string()));
    assert_eq!(db.get_message_role("s1", 9999).unwrap(), None);
    db.close();
}

// =====================================================================
// batch writer — tests/hermes_state/test_append_messages_batch.py
// =====================================================================

fn turn_messages() -> Vec<MessageInput> {
    vec![
        msg("user", "question"),
        MessageInput {
            role: "assistant".into(),
            content: Some(json!("let me check")),
            tool_calls: Some(json!([{"name": "terminal", "arguments": "{}"}])),
            reasoning_content: Some("thinking...".into()),
            finish_reason: Some("tool_calls".into()),
            ..Default::default()
        },
        MessageInput {
            role: "tool".into(),
            content: Some(json!("tool output")),
            tool_name: Some("terminal".into()),
            tool_call_id: Some("call_1".into()),
            ..Default::default()
        },
        MessageInput {
            role: "assistant".into(),
            content: Some(json!("answer")),
            finish_reason: Some("stop".into()),
            ..Default::default()
        },
    ]
}

#[test]
fn batch_rows_identical_to_single_appends() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path.clone()), false).expect("open");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    let (_dir2, path2) = tmp_db("state2.db");
    let db2 = SessionDB::open(Some(path2.clone()), false).expect("open2");
    db2.create_session("sess-batch", "cli", &NewSession::default()).expect("create2");
    {
        let msgs = turn_messages();
        db.append_messages_batch("sess-batch", &msgs, None, None).expect("batch");
        for m in &msgs {
            let role = m.role.clone();
            db2.append_message(
                "sess-batch",
                &MessageInput {
                    role: role.clone(),
                    content: m.content.clone(),
                    tool_name: m.tool_name.clone(),
                    tool_calls: m.tool_calls.clone(),
                    tool_call_id: m.tool_call_id.clone(),
                    finish_reason: m.finish_reason.clone(),
                    reasoning_content: if role == "assistant" { m.reasoning_content.clone() } else { None },
                    ..Default::default()
                },
                None,
            )
            .expect("single");
        }
    }
    let cols = "role, content, tool_call_id, tool_calls, tool_name, \
                finish_reason, reasoning_content, observed, active";
    let conn_a = Connection::open(&path).unwrap();
    let conn_b = Connection::open(&path2).unwrap();
    type MsgCols = (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i64, i64);
    let rows_a: Vec<MsgCols> = conn_a
        .prepare(&format!("SELECT {cols} FROM messages ORDER BY id"))
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let rows_b: Vec<MsgCols> = conn_b
        .prepare(&format!("SELECT {cols} FROM messages ORDER BY id"))
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows_a, rows_b);
    db.close();
    db2.close();
}

#[test]
fn batch_reasoning_gated_to_assistant_rows() {
    let (_dir, db) = open_db("state.db");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    db.append_messages_batch(
        "sess-batch",
        &[MessageInput {
            role: "tool".into(),
            content: Some(json!("out")),
            tool_name: Some("t".into()),
            tool_call_id: Some("c1".into()),
            reasoning_content: Some("should not persist".into()),
            ..Default::default()
        }],
        None,
        None,
    )
    .expect("batch");
    let r: Option<String> = db
        .writer_conn()
        .query_row("SELECT reasoning_content FROM messages", [], |r| r.get(0))
        .unwrap();
    assert!(r.is_none());
    db.close();
}

#[test]
fn batch_counters_aggregate_once() {
    let (_dir, db) = open_db("state.db");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    db.append_messages_batch("sess-batch", &turn_messages(), None, None).expect("batch");
    let row = db
        .writer_conn()
        .query_row("SELECT message_count, tool_call_count FROM sessions WHERE id = ?", ["sess-batch"], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
        .unwrap();
    assert_eq!(row, (4, 1));
    db.close();
}

#[test]
fn batch_returns_inserted_count_and_empty_is_noop() {
    let (_dir, db) = open_db("state.db");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    assert_eq!(db.append_messages_batch("sess-batch", &turn_messages(), None, None).unwrap(), 4);
    assert_eq!(db.append_messages_batch("sess-batch", &[], None, None).unwrap(), 0);
    let c: i64 = db.writer_conn().query_row("SELECT message_count FROM sessions WHERE id = ?", ["sess-batch"], |r| r.get(0)).unwrap();
    assert_eq!(c, 4);
    db.close();
}

#[test]
fn batch_atomicity_all_or_nothing() {
    // The row serializer is run inside one BEGIN IMMEDIATE; a mid-batch
    // failure leaves zero rows and untouched counters. We force the failure
    // by making the _insert_message_rows equivalent raise on row 3 via a
    // trigger that rejects a tool row with 'boom'.
    let (_dir, db) = open_db("state.db");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    db.writer_conn()
        .execute_batch(
            "CREATE TRIGGER boom_on_tool
             BEFORE INSERT ON messages
             WHEN NEW.role = 'tool' AND NEW.tool_call_id = 'call_1'
             BEGIN SELECT RAISE(ABORT, 'boom mid-batch'); END",
        )
        .unwrap();
    // A non-empty batch must now fail atomically (ignore the inserted row
    // count; the error propagates as a string).
    let res = db.append_messages_batch("sess-batch", &turn_messages(), None, None);
    assert!(res.is_err(), "mid-batch failure must propagate");
    let count: i64 = db.writer_conn().query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
    let row = db.writer_conn().query_row("SELECT message_count, tool_call_count FROM sessions WHERE id = ?", ["sess-batch"], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))).unwrap();
    assert_eq!(row, (0, 0));
    db.close();
}

#[test]
fn batch_tool_calls_json_string_not_double_encoded() {
    let (_dir, db) = open_db("state.db");
    db.create_session("sess-batch", "cli", &NewSession::default()).expect("create");
    db.append_messages_batch(
        "sess-batch",
        &[MessageInput {
            role: "assistant".into(),
            content: Some(json!("x")),
            tool_calls: Some(json!([{"name": "t", "arguments": "{}"}])),
            ..Default::default()
        }],
        None,
        None,
    )
    .expect("batch");
    let raw: String = db.writer_conn().query_row("SELECT tool_calls FROM messages", [], |r| r.get(0)).unwrap();
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed, json!([{"name": "t", "arguments": "{}"}]));
    db.close();
}

// =====================================================================
// titles
// =====================================================================

#[test]
fn set_and_get_title() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    assert!(db.set_session_title("s1", "My Session").unwrap());
    let session = db.get_session("s1").unwrap().unwrap();
    assert_eq!(session.title.as_deref(), Some("My Session"));
    assert_eq!(db.get_session_title("s1").unwrap().as_deref(), Some("My Session"));
    db.close();
}

#[test]
fn title_empty_string_normalized_to_none() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.set_session_title("s1", "My Title").unwrap();
    db.set_session_title("s1", "").unwrap();
    let session = db.get_session("s1").unwrap().unwrap();
    assert!(session.title.is_none());
    db.close();
}

#[test]
fn sanitize_title_normal_unchanged() {
    assert_eq!(SessionDB::sanitize_title("My Project").unwrap(), Some("My Project".to_string()));
}

#[test]
fn sanitize_title_control_chars_stripped() {
    assert_eq!(SessionDB::sanitize_title("hello\x00world").unwrap(), Some("helloworld".to_string()));
    assert_eq!(SessionDB::sanitize_title("\x07\x08test\x1b").unwrap(), Some("test".to_string()));
    assert_eq!(SessionDB::sanitize_title("  spaced  out  ").unwrap(), Some("spaced out".to_string()));
}

#[test]
fn sanitize_title_exceeds_max_length_raises() {
    let title = "A".repeat(101);
    let err = SessionDB::sanitize_title(&title).unwrap_err();
    assert!(err.to_string().contains("too long"), "err: {}", err);
}

#[test]
fn title_unrelated_session_still_conflicts() {
    let (_dir, db) = open_db("state.db");
    db.create_session("a", "cli", &NewSession::default()).expect("create a");
    db.create_session("b", "cli", &NewSession::default()).expect("create b");
    db.set_session_title("a", "shared").unwrap();
    let err = db.set_session_title("b", "shared").unwrap_err();
    assert!(err.to_string().contains("already in use"), "err: {}", err);
    assert_eq!(db.get_session("a").unwrap().unwrap().title.as_deref(), Some("shared"));
    db.close();
}

#[test]
fn title_resolve_exact_and_nonexistent() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.set_session_title("s1", "my project").unwrap();
    assert_eq!(db.resolve_session_by_title("my project").unwrap().as_deref(), Some("s1"));
    assert_eq!(db.resolve_session_by_title("nonexistent").unwrap(), None);
    db.close();
}

#[test]
fn next_title_no_existing() {
    let (_dir, db) = open_db("state.db");
    assert_eq!(db.get_next_title_in_lineage("my project").unwrap(), "my project");
    db.close();
}

#[test]
fn title_resolve_with_underscore_is_literal() {
    // TestTitleSqlWildcards.test_resolve_title_with_underscore
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");
    db.set_session_title("s1", "test_project").unwrap();
    db.create_session("s2", "cli", &NewSession::default()).expect("create2");
    db.set_session_title("s2", "testXproject #2").unwrap();
    assert_eq!(db.resolve_session_by_title("test_project").unwrap().as_deref(), Some("s1"));
    db.close();
}

#[test]
fn next_title_increments_numbered_variants() {
    let (_dir, db) = open_db("state.db");
    db.create_session("a", "cli", &NewSession::default()).expect("create a");
    db.set_session_title("a", "my session").unwrap();
    db.create_session("b", "cli", &NewSession::default()).expect("create b");
    db.set_session_title("b", "my session #2").unwrap();
    db.create_session("c", "cli", &NewSession::default()).expect("create c");
    db.set_session_title("c", "my session #7").unwrap();
    assert_eq!(db.get_next_title_in_lineage("my session").unwrap(), "my session #8");
    // Previous numbered variant input still finds base.
    assert_eq!(db.get_next_title_in_lineage("my session #7").unwrap(), "my session #8");
    db.close();
}

#[test]
fn resolve_session_id_exact_and_prefix() {
    let (_dir, db) = open_db("state.db");
    db.create_session("abc123", "cli", &NewSession::default()).expect("create");
    db.create_session("abc456", "cli", &NewSession::default()).expect("create2");
    assert_eq!(db.resolve_session_id("abc123").unwrap().as_deref(), Some("abc123"));
    // Ambiguous prefix -> None (two matches).
    assert_eq!(db.resolve_session_id("abc").unwrap(), None);
    // Unique prefix on a single session DB.
    let (_dir2, db2) = open_db("state2.db");
    db2.create_session("unique-sid", "cli", &NewSession::default()).expect("create3");
    assert_eq!(db2.resolve_session_id("unique").unwrap().as_deref(), Some("unique-sid"));
    assert_eq!(db2.resolve_session_id("zzz").unwrap(), None);
    db.close();
    db2.close();
}

#[test]
fn rename_continuation_back_to_base_transfers_title() {
    // TestSessionTitleLineage._make_compression_chain + transfer.
    let (_dir, db) = open_db("state.db");
    let t0 = 1_700_000_000.0;
    db.create_session("root", "cli", &NewSession::default()).expect("create root");
    db.writer_conn()
        .execute("UPDATE sessions SET started_at=? WHERE id=?", rusqlite::params![t0, "root"])
        .unwrap();
    db.writer_conn()
        .execute("UPDATE sessions SET ended_at=?, end_reason='compression' WHERE id=?", rusqlite::params![t0 + 100.0, "root"])
        .unwrap();
    db.create_session("tip", "cli", &NewSession { parent_session_id: Some("root".into()), ..Default::default() }).expect("create tip");
    db.writer_conn()
        .execute("UPDATE sessions SET started_at=? WHERE id=?", rusqlite::params![t0 + 200.0, "tip"])
        .unwrap();

    db.set_session_title("root", "fingerprint-scanner").unwrap();
    db.set_session_title("tip", "fingerprint-scanner #2").unwrap();
    assert!(db.set_session_title("tip", "fingerprint-scanner").unwrap());
    assert_eq!(db.get_session("tip").unwrap().unwrap().title.as_deref(), Some("fingerprint-scanner"));
    assert!(db.get_session("root").unwrap().unwrap().title.is_none());
    db.close();
}

// =====================================================================
// system prompt + parent backfill
// =====================================================================

#[test]
fn system_prompt_stored_by_hash_and_resolved_on_read() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "s1",
        "cli",
        &NewSession {
            system_prompt: Some("you are hermes".into()),
            ..Default::default()
        },
    )
    .expect("create");
    let session = db.get_session("s1").unwrap().unwrap();
    assert_eq!(session.system_prompt.as_deref(), Some("you are hermes"));
    // Re-created with no system prompt keeps the stored one (COALESCE).
    db.create_session("s1", "cli", &NewSession::default()).expect("recreate");
    let session = db.get_session("s1").unwrap().unwrap();
    assert_eq!(session.system_prompt.as_deref(), Some("you are hermes"));
    db.close();
}

#[test]
fn create_session_backfills_cwd_from_parent() {
    // _insert_session_row parent backfill: child inherits cwd/git_repo_root
    // when its own are NULL.
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "parent",
        "cli",
        &NewSession {
            cwd: Some("/work/repo".into()),
            git_repo_root: Some("/work/repo".into()),
            ..Default::default()
        },
    )
    .expect("create parent");
    db.create_session(
        "child",
        "cli",
        &NewSession { parent_session_id: Some("parent".into()), ..Default::default() },
    )
    .expect("create child");
    let child = db.get_session("child").unwrap().unwrap();
    assert_eq!(child.cwd.as_deref(), Some("/work/repo"));
    assert_eq!(child.git_repo_root.as_deref(), Some("/work/repo"));
    // Explicit child values are never overwritten.
    db.create_session(
        "child2",
        "cli",
        &NewSession {
            parent_session_id: Some("parent".into()),
            cwd: Some("/other".into()),
            ..Default::default()
        },
    )
    .expect("create child2");
    assert_eq!(db.get_session("child2").unwrap().unwrap().cwd.as_deref(), Some("/other"));
    db.close();
}
