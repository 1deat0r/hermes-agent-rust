//! Parity oracles for the session deletion + maintenance surface, mirroring
//! upstream tests/test_hermes_state.py (TestCounts.message_count,
//! TestDeleteAndExport.delete_session, delete_session expected-targets
//! fail-closed) plus the empty-session reap / marker purge / kanban retag /
//! auto-maintenance contracts @ b9aa928.

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

fn create(db: &SessionDB, sid: &str, source: &str) {
    db.create_session(sid, source, &NewSession::default()).expect("create");
}

fn append(db: &SessionDB, sid: &str, role: &str, content: &str) {
    db.append_message(sid, &msg(role, content), None).expect("append");
}

// =====================================================================
// TestCounts.message_count + platform id dedupe
// =====================================================================

#[test]
fn message_count_total_and_per_session() {
    let (_dir, db) = open_db("state.db");
    assert_eq!(db.message_count(None).expect("empty"), 0);
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "Hello");
    append(&db, "s1", "assistant", "Hi");
    assert_eq!(db.message_count(None).expect("total"), 2);
    assert_eq!(db.message_count(Some("s1")).expect("s1"), 2);
    assert_eq!(db.message_count(Some("nope")).expect("nope"), 0);
}

#[test]
fn has_platform_message_id_dedupe_guard() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "telegram");
    db.append_message("s1", &MessageInput {
        role: "user".into(),
        content: Some(json!("ping")),
        platform_message_id: Some("tg-42".into()),
        ..Default::default()
    }, None).expect("append");
    assert!(db.has_platform_message_id("s1", "tg-42").expect("hit"));
    assert!(!db.has_platform_message_id("s1", "tg-43").expect("miss"));
    assert!(!db.has_platform_message_id("other", "tg-42").expect("other"));
}

// =====================================================================
// TestDeleteAndExport — delete_session family
// =====================================================================

#[test]
fn delete_session_removes_row_and_messages() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "Hello");
    assert!(db.delete_session("s1", None, None).expect("delete"));
    assert!(db.get_session("s1").expect("gone").is_none());
    assert_eq!(db.message_count(Some("s1")).expect("count"), 0);
    // Missing session returns false, not error.
    assert!(!db.delete_session("s1", None, None).expect("missing"));
}

#[test]
fn delete_session_cascades_delegate_and_orphans_branch() {
    let (_dir, db) = open_db("state.db");
    create(&db, "parent", "cli");
    db.create_session("delegate", "cli", &NewSession {
        parent_session_id: Some("parent".into()),
        model_config: Some(json!({"_delegate_from": "parent"})),
        ..Default::default()
    })
    .expect("delegate");
    db.create_session("branch", "cli", &NewSession {
        parent_session_id: Some("parent".into()),
        model_config: Some(json!({"_branched_from": "parent"})),
        ..Default::default()
    })
    .expect("branch");

    let targets = db.get_session_delete_targets("parent").expect("targets");
    assert_eq!(targets, vec!["parent".to_string(), "delegate".to_string()]);

    assert!(db.delete_session("parent", None, None).expect("delete"));
    assert!(db.get_session("parent").expect("parent gone").is_none());
    assert!(db.get_session("delegate").expect("delegate gone").is_none());
    // Branch survives, orphaned (parent_session_id NULL).
    let branch = db.get_session("branch").expect("branch").expect("some");
    assert_eq!(branch.parent_session_id, None);
}

#[test]
fn delete_session_expected_targets_fail_closed_on_new_delegate() {
    let (_dir, db) = open_db("state.db");
    create(&db, "parent", "cli");
    let delegate = |_sid: &str| NewSession {
        parent_session_id: Some("parent".into()),
        model_config: Some(json!({"_delegate_from": "parent"})),
        ..Default::default()
    };
    db.create_session("delegate", "cli", &delegate("delegate")).expect("delegate");
    db.create_session("branch", "cli", &NewSession {
        parent_session_id: Some("parent".into()),
        model_config: Some(json!({"_branched_from": "parent"})),
        ..Default::default()
    })
    .expect("branch");

    let expected_ids = db.get_session_delete_targets("parent").expect("targets");
    assert_eq!(expected_ids, vec!["parent".to_string(), "delegate".to_string()]);

    db.create_session("late-delegate", "cli", &delegate("late-delegate")).expect("late");

    assert!(
        !db.delete_session("parent", None, Some(&expected_ids)).expect("fail closed")
    );
    assert!(db.get_session("parent").expect("parent").is_some());
    assert!(db.get_session("delegate").expect("delegate").is_some());
    assert!(db.get_session("late-delegate").expect("late").is_some());
    assert!(db.get_session("branch").expect("branch").is_some());
}

#[test]
fn delete_session_if_empty_only_when_no_resumable_content() {
    let (_dir, db) = open_db("state.db");
    // Empty session (no messages/title/children) is deleted.
    create(&db, "quiet", "cli");
    assert!(db.delete_session_if_empty("quiet", None).expect("delete"));
    assert!(db.get_session("quiet").expect("gone").is_none());

    // A session with a message is preserved.
    create(&db, "talker", "cli");
    append(&db, "talker", "user", "hi");
    assert!(!db.delete_session_if_empty("talker", None).expect("keep"));
    assert!(db.get_session("talker").expect("kept").is_some());

    // A titled session is preserved even with no messages.
    create(&db, "titled", "cli");
    db.set_session_title("titled", "A real title").expect("title");
    // A titled session is preserved even with no messages.
    create(&db, "titled", "cli");
    db.set_session_title("titled", "A real title").expect("title");
    assert!(!db.delete_session_if_empty("titled", None).expect("keep title"));

    // A parent with a child is preserved.
    create(&db, "outer", "cli");
    db.create_session("inner", "cli", &NewSession {
        parent_session_id: Some("outer".into()),
        ..Default::default()
    })
    .expect("inner");
    assert!(!db.delete_session_if_empty("outer", None).expect("keep child"));
}

#[test]
fn delete_sessions_bulk_transactional() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    create(&db, "s2", "cli");
    create(&db, "s3", "cli");
    append(&db, "s1", "user", "x");
    // Delegate child of s2 cascade-deletes too.
    db.create_session("s2-d", "cli", &NewSession {
        parent_session_id: Some("s2".into()),
        model_config: Some(json!({"_delegate_from": "s2"})),
        ..Default::default()
    })
    .expect("s2d");

    let count = db
        .delete_sessions(&["s1".to_string(), "s2".to_string(), "missing".to_string()], None)
        .expect("bulk");
    assert_eq!(count, 2);
    assert!(db.get_session("s1").expect("s1 gone").is_none());
    assert!(db.get_session("s2").expect("s2 gone").is_none());
    assert!(db.get_session("s2-d").expect("s2d gone").is_none());
    assert!(db.get_session("s3").expect("s3 kept").is_some());
}

#[test]
fn delete_empty_sessions_reaps_only_empty_ended() {
    let (_dir, db) = open_db("state.db");
    create(&db, "empty_ended", "cli");
    db.end_session("empty_ended", "tui_shutdown").expect("end");
    create(&db, "live_empty", "cli"); // not ended
    create(&db, "has_msgs", "cli");
    db.end_session("has_msgs", "tui_shutdown").expect("end2");
    append(&db, "has_msgs", "user", "content");
    create(&db, "archived_empty", "cli");
    db.end_session("archived_empty", "tui_shutdown").expect("end3");
    db.set_session_archived("archived_empty", true).expect("archive");

    assert_eq!(db.count_empty_sessions().expect("count"), 1);
    let deleted = db.delete_empty_sessions(None).expect("delete empties");
    assert_eq!(deleted, 1);
    assert!(db.get_session("empty_ended").expect("gone").is_none());
    assert!(db.get_session("live_empty").expect("kept live").is_some());
    assert!(db.get_session("has_msgs").expect("kept msgs").is_some());
    assert!(db.get_session("archived_empty").expect("kept archived").is_some());
}

#[test]
fn purge_stale_tool_call_markers_dry_run_and_write() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    let markers = |_sid: &str| MessageInput {
        role: "assistant".into(),
        content: Some(json!("[memory]")),
        tool_calls: Some(json!([{"id": "c1", "type": "function"}])),
        ..Default::default()
    };
    let clean = |_sid: &str| MessageInput {
        role: "assistant".into(),
        content: Some(json!("real answer")),
        tool_calls: Some(json!([{"id": "c2", "type": "function"}])),
        ..Default::default()
    };
    append(&db, "s1", "user", "do");
    db.append_message("s1", &markers("s1"), None).expect("marker");
    db.append_message("s1", &clean("s1"), None).expect("clean");

    let dry = db.purge_stale_tool_call_markers(true, true).expect("dry");
    assert_eq!(dry["dry_run"].as_bool(), Some(true));
    assert_eq!(dry["rows_affected"].as_i64(), Some(1));
    let ids = dry["row_ids"].as_array().unwrap();
    let marker_id: i64 = {
        let conn = db.writer_conn();
        conn.query_row(
            "SELECT id FROM messages WHERE content = '[memory]'",
            [],
            |r| r.get(0),
        )
        .expect("marker id")
    };
    assert!(ids.iter().any(|v| v.as_i64() == Some(marker_id)));

    let result = db.purge_stale_tool_call_markers(false, false).expect("purge");
    assert_eq!(result["rows_affected"].as_i64(), Some(1));
    assert_eq!(result["backup_path"], Value::Null);
    // Content cleared; tool_calls untouched; clean message untouched.
    let conn = db.writer_conn();
    let (content, tool_calls): (String, String) = conn
        .query_row(
            "SELECT content, tool_calls FROM messages WHERE id = ?",
            rusqlite::params![marker_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("row");
    assert_eq!(content, "");
    assert!(tool_calls.contains("c1"));
    // Second run no-ops.
    let result = db.purge_stale_tool_call_markers(false, true).expect("purge2");
    // backup=true with nothing to change takes no snapshot.
    assert_eq!(result["rows_affected"].as_i64(), Some(0));
    assert_eq!(result["backup_path"], Value::Null);
}

#[test]
fn retag_kanban_worker_sessions_gated_once() {
    let (_dir, db) = open_db("state.db");
    let workspaces = "/tmp/boards".to_string();
    create(&db, "w1", "cli");
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET cwd = ? WHERE id = 'w1'",
        rusqlite::params![format!("{workspaces}/board-a")],
    )
    .expect("cwd");
    create(&db, "other", "cli");
    // outside the workspace -> untouched
    conn.execute(
        "UPDATE sessions SET cwd = '/elsewhere' WHERE id = 'other'",
        [],
    )
    .expect("cwd2");
    drop(conn);

    let retagged = db.retag_kanban_worker_sessions(&workspaces).expect("retag");
    assert_eq!(retagged, 1);
    let src: String = {
        let conn = db.writer_conn();
        conn.query_row("SELECT source FROM sessions WHERE id = 'w1'", [], |r| r.get(0))
            .expect("src")
    };
    assert_eq!(src, "kanban");
    // Gated: second call no-ops.
    assert_eq!(db.retag_kanban_worker_sessions(&workspaces).expect("gated"), 0);
    // Untouched session stays cli.
    let src: String = {
        let conn = db.writer_conn();
        conn.query_row("SELECT source FROM sessions WHERE id = 'other'", [], |r| r.get(0))
            .expect("src2")
    };
    assert_eq!(src, "cli");
    // Empty root short-circuits.
    assert_eq!(db.retag_kanban_worker_sessions("").expect("empty"), 0);
}

#[test]
fn logical_size_bytes_and_vacuum() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "some content to fill a page");
    let size = db.logical_size_bytes();
    assert!(size.unwrap_or(0) > 0);
    let optimized = db.vacuum().expect("vacuum");
    assert!(optimized >= 0);
    let after = db.logical_size_bytes();
    assert!(after.is_some() && after.unwrap() > 0);
}

#[test]
fn maybe_auto_prune_and_vacuum_idempotent() {
    let (_dir, db) = open_db("state.db");
    // Fresh DB: runs, prunes nothing, records the attempt.
    let first = db
        .maybe_auto_prune_and_vacuum(90, 24, true, None, 30)
        .expect("first");
    assert_eq!(first["skipped"].as_bool(), Some(false));
    assert_eq!(first["pruned"].as_i64(), Some(0));
    // Second call within the interval is skipped.
    let second = db
        .maybe_auto_prune_and_vacuum(90, 24, true, None, 30)
        .expect("second");
    assert_eq!(second["skipped"].as_bool(), Some(true));

    // Older-than-retention ended sessions get pruned + vacuumed.
    let (_dir2, db2) = open_db("prune.db");
    create(&db2, "old", "cli");
    append(&db2, "old", "user", "x");
    db2.end_session("old", "tui_shutdown").expect("end");
    let conn = db2.writer_conn();
    conn.execute(
        "UPDATE sessions SET started_at = 1.0 WHERE id = 'old'",
        [],
    )
    .expect("aged");
    conn.execute(
        "UPDATE messages SET timestamp = 1.0 WHERE session_id = 'old'",
        [],
    )
    .expect("aged msgs");
    conn.execute("DELETE FROM state_meta WHERE key = 'last_auto_prune'", []).expect("reset");
    drop(conn);
    let r = db2
        .maybe_auto_prune_and_vacuum(90, 24, false, None, 30)
        .expect("prune run");
    assert!(r["pruned"].as_i64().unwrap_or(0) >= 1);
}

#[test]
fn maybe_auto_archive_idempotent_and_archives_idle() {
    let (_dir, db) = open_db("state.db");
    let first = db.maybe_auto_archive(3.0, 24, true).expect("first");
    assert_eq!(first["skipped"].as_bool(), Some(false));
    let second = db.maybe_auto_archive(3.0, 24, true).expect("second");
    assert_eq!(second["skipped"].as_bool(), Some(true));

    let (_dir2, db2) = open_db("archive.db");
    create(&db2, "idle", "cli");
    append(&db2, "idle", "user", "content");
    {
        let conn = db2.writer_conn();
        conn.execute(
            "UPDATE sessions SET started_at = 1.0 WHERE id = 'idle'",
            [],
        )
        .expect("aged");
        conn.execute(
            "UPDATE messages SET timestamp = 1.0 WHERE session_id = 'idle'",
            [],
        )
        .expect("aged msgs");
        conn.execute("DELETE FROM state_meta WHERE key = 'last_auto_archive'", []).expect("reset");
    }
    let r = db2.maybe_auto_archive(0.1, 24, true).expect("archive run");
    assert_eq!(r["archived"].as_i64(), Some(1));
    let row = db2.get_session("idle").expect("get").expect("row");
    assert!(row.archived);
}

#[test]
fn clear_messages_resets_counters() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "one");
    append(&db, "s1", "assistant", "two");
    db.clear_messages("s1").expect("clear");
    assert_eq!(db.message_count(Some("s1")).expect("count"), 0);
    let row = db.get_session("s1").expect("get").expect("row");
    assert_eq!(row.message_count, 0);
    assert_eq!(row.tool_call_count, 0);
}

#[test]
fn finalize_orphaned_compression_sessions_ends_unfinished_children() {
    let (_dir, db) = open_db("state.db");
    let base = 1_700_000_000.0;
    create(&db, "parent", "cli");
    append(&db, "parent", "user", "work");
    {
        let conn = db.writer_conn();
        conn.execute(
            "UPDATE sessions SET started_at = ?, ended_at = ?, end_reason = 'compression' WHERE id = 'parent'",
            rusqlite::params![base, base + 10.0],
        )
        .expect("end parent");
    }
    // Orphaned child: has messages, never ended, api_call_count=0, started
    // more than 7 days ago.
    db.create_session("orphan", "cli", &NewSession {
        parent_session_id: Some("parent".into()),
        ..Default::default()
    })
    .expect("orphan");
    append(&db, "orphan", "assistant", "post-compression reply");
    {
        let conn = db.writer_conn();
        conn.execute(
            "UPDATE sessions SET started_at = 1.0 WHERE id = 'orphan'",
            [],
        )
        .expect("aged");
    }
    // A fresh unlucky child (started recently) must NOT be finalized.
    db.create_session("fresh", "cli", &NewSession {
        parent_session_id: Some("parent".into()),
        ..Default::default()
    })
    .expect("fresh");
    append(&db, "fresh", "assistant", "recent");

    let finalized = db.finalize_orphaned_compression_sessions().expect("finalize");
    assert_eq!(finalized, 1);
    let orphan = db.get_session("orphan").expect("get").expect("row");
    assert_eq!(orphan.end_reason.as_deref(), Some("orphaned_compression"));
    assert!(orphan.ended_at.is_some());
    let fresh = db.get_session("fresh").expect("get2").expect("row");
    assert_eq!(fresh.end_reason, None);
}
