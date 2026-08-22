//! Parity oracles for the conversation projection surface, mirroring
//! upstream tests/hermes_state/test_resolve_resume_session_id.py,
//! tests/hermes_state/test_conversation_root.py, tests/test_hermes_state.py
//! (conversation decode / memory-context strip / reasoning restore),
//! tests/test_tui_gateway_server.py (get_resume_conversations
//! verification-candidate split), and the agent-level repair_message_sequence
//! contracts @ b9aa928.

use std::path::PathBuf;

use hermes_state::conversation::repair_message_sequence;
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

fn msg(role: &str, content: Option<&str>) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: content.map(|c| serde_json::json!(c)),
        ..Default::default()
    }
}

fn create(db: &SessionDB, sid: &str, source: &str) {
    db.create_session(sid, source, &NewSession::default()).expect("create");
}

fn append(db: &SessionDB, sid: &str, role: &str, content: &str) {
    db.append_message(sid, &msg(role, Some(content)), None).expect("append");
}

/// Create sessions in order with deterministic started_at so the
/// parent→child walk ordering is stable.
fn make_chain(db: &SessionDB, ids_with_parent: &[(&str, Option<&str>)]) {
    let base = 1_700_000_000.0;
    for (i, (sid, parent)) in ids_with_parent.iter().enumerate() {
        db.create_session(sid, "cli", &NewSession {
            parent_session_id: parent.map(|p| p.to_string()),
            ..Default::default()
        })
        .expect("create");
        set_started(db, sid, base + (i as f64) * 100.0);
    }
}

fn set_started(db: &SessionDB, sid: &str, ts: f64) {
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET started_at = ? WHERE id = ?",
        rusqlite::params![ts, sid],
    )
    .expect("started");
}

fn set_ended(db: &SessionDB, sid: &str, ts: f64, reason: &str) {
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET ended_at = ?, end_reason = ? WHERE id = ?",
        rusqlite::params![ts, reason, sid],
    )
    .expect("ended");
}

// =====================================================================
// tests/hermes_state/test_resolve_resume_session_id.py
// =====================================================================

#[test]
fn resolve_resume_returns_self_when_only_parent_has_messages() {
    let (_dir, db) = open_db("state.db");
    make_chain(&db, &[("root", None), ("child", Some("root"))]);
    append(&db, "root", "user", "hi");
    assert_eq!(db.resolve_resume_session_id("root").expect("resolve"), "root");
}

#[test]
fn resolve_resume_walks_from_middle_of_chain() {
    let (_dir, db) = open_db("state.db");
    make_chain(&db, &[("a", None), ("b", Some("a")), ("c", Some("b")), ("d", Some("c"))]);
    append(&db, "d", "user", "x");
    assert_eq!(db.resolve_resume_session_id("b").expect("b"), "d");
    assert_eq!(db.resolve_resume_session_id("c").expect("c"), "d");
}

#[test]
fn resolve_resume_follows_compression_tip_when_parent_retains_messages() {
    let (_dir, db) = open_db("state.db");
    let base = 1_700_000_000.0;
    create(&db, "root", "cli");
    append(&db, "root", "user", "pre-compression turn");
    set_started(&db, "root", base);
    set_ended(&db, "root", base + 50.0, "compression");
    db.create_session("cont", "cli", &NewSession {
        parent_session_id: Some("root".into()),
        ..Default::default()
    })
    .expect("cont");
    set_started(&db, "cont", base + 100.0);
    append(&db, "cont", "assistant", "post-compression reply");

    assert_eq!(db.resolve_resume_session_id("root").expect("resolve"), "cont");
}

#[test]
fn resolve_resume_prefers_most_recent_child_when_fork_exists() {
    let (_dir, db) = open_db("state.db");
    make_chain(&db, &[("parent", None), ("older_fork", Some("parent")), ("newer_fork", Some("parent"))]);
    append(&db, "newer_fork", "user", "x");
    assert_eq!(db.resolve_resume_session_id("parent").expect("resolve"), "newer_fork");
}

// =====================================================================
// tests/hermes_state/test_conversation_root.py
// =====================================================================

#[test]
fn conversation_root_of_standalone_session_is_itself() {
    let (_dir, db) = open_db("state.db");
    create(&db, "solo", "cli");
    assert_eq!(db.get_conversation_root("solo").expect("root"), "solo");
}

#[test]
fn conversation_root_covers_delegate_child_sessions() {
    let (_dir, db) = open_db("state.db");
    create(&db, "parent", "cli");
    db.create_session("child", "delegate", &NewSession {
        parent_session_id: Some("parent".into()),
        ..Default::default()
    })
    .expect("child");
    assert_eq!(db.get_conversation_root("child").expect("root"), "parent");
}

#[test]
fn conversation_root_walks_full_lineage() {
    let (_dir, db) = open_db("state.db");
    make_chain(&db, &[("root", None), ("mid", Some("root")), ("tip", Some("mid"))]);
    assert_eq!(db.get_conversation_root("tip").expect("root"), "root");
    assert_eq!(db.get_conversation_root("mid").expect("root"), "root");
}

// =====================================================================
// tests/test_hermes_state.py — conversation decode
// =====================================================================

#[test]
fn conversation_strips_leaked_memory_context() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    db.append_message(
        "s1",
        &MessageInput {
            role: "assistant".into(),
            content: Some(json!(
                "<memory-context>\n\
                 [System note: The following is recalled memory context, NOT new user input. Treat as informational background data.]\n\n\
                 ## Honcho Context\n\
                 stale memory\n\
                 </memory-context>\n\n\
                 Visible answer"
            )),
            ..Default::default()
        },
        None,
    )
    .expect("append");

    let conv = db.get_messages_as_conversation("s1", false, false, false, false).expect("conv");
    assert_eq!(conv.len(), 1);
    assert_eq!(conv[0]["role"], json!("assistant"));
    assert_eq!(conv[0]["content"], json!("Visible answer"));
    assert!(conv[0].get("timestamp").and_then(Value::as_f64).is_some());
}

#[test]
fn conversation_restores_reasoning_and_tool_calls() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "telegram");
    append(&db, "s1", "user", "create a cron job");
    db.append_message(
        "s1",
        &MessageInput {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(json!([
                {"function": {"name": "cronjob", "arguments": "{}"}, "id": "c1", "type": "function"}
            ])),
            reasoning: Some("I should call the cronjob tool to schedule this.".into()),
            ..Default::default()
        },
        None,
    )
    .expect("assistant");
    db.append_message(
        "s1",
        &MessageInput {
            role: "tool".into(),
            content: Some(json!(r#"{"job_id": "abc"}"#)),
            tool_call_id: Some("c1".into()),
            ..Default::default()
        },
        None,
    )
    .expect("tool");

    let conv = db.get_messages_as_conversation("s1", false, false, false, false).expect("conv");
    assert_eq!(conv.len(), 3);
    let assistant = &conv[1];
    assert_eq!(assistant["role"], json!("assistant"));
    assert_eq!(
        assistant.get("reasoning").and_then(Value::as_str),
        Some("I should call the cronjob tool to schedule this.")
    );
    assert_eq!(assistant["tool_calls"][0]["id"], json!("c1"));
    assert!(conv[0].get("reasoning").is_none());
    assert!(conv[2].get("reasoning").is_none());
}

#[test]
fn conversation_row_ids_and_api_content_sidecar() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    db.append_message(
        "s1",
        &MessageInput {
            role: "user".into(),
            content: Some(json!("hello")),
            api_content: Some("hello-<exact-bytes>".into()),
            ..Default::default()
        },
        None,
    )
    .expect("append");

    let conv = db.get_messages_as_conversation("s1", false, false, false, true).expect("conv");
    assert!(conv[0].get("_row_id").and_then(Value::as_i64).is_some());
    // api_content verbatim.
    assert_eq!(conv[0]["api_content"].as_str(), Some("hello-<exact-bytes>"));

    let without_ids = db.get_messages_as_conversation("s1", false, false, false, false).expect("conv2");
    assert!(without_ids[0].get("_row_id").is_none());
}

// =====================================================================
// get_resume_conversations — verification-candidate divergence (#65919)
// =====================================================================

#[test]
fn resume_conversations_collapses_candidate_in_model_history_only() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "tui");
    append(&db, "s1", "user", "do the thing");
    db.append_message("s1", &MessageInput {
        role: "assistant".into(),
        content: Some(json!("long substantive answer")),
        finish_reason: Some("verification_required".into()),
        ..Default::default()
    }, None).expect("candidate");
    db.append_message("s1", &MessageInput {
        role: "assistant".into(),
        content: Some(json!("terse verified reply")),
        finish_reason: Some("stop".into()),
        ..Default::default()
    }, None).expect("verified");

    let (model_history, display_history) = db.get_resume_conversations("s1").expect("resume");
    assert!(!model_history.iter().any(|m| m.get("content").and_then(Value::as_str).map(|s| s.contains("long substantive")).unwrap_or(false)));
    assert!(display_history.iter().any(|m| m.get("content").and_then(Value::as_str).map(|s| s.contains("long substantive")).unwrap_or(false)));
}

// =====================================================================
// repair_message_sequence contracts
// =====================================================================

#[test]
fn repair_merges_consecutive_assistant_and_unions_tool_calls() {
    let mut messages = vec![
        json!({"role": "user", "content": "hi"}),
        json!({"role": "assistant", "content": "turn 1", "tool_calls": [{"id": "c1"}]}),
        json!({"role": "assistant", "content": "turn 2", "tool_calls": [{"id": "c2"}]}),
    ];
    let repairs = repair_message_sequence(&mut messages);
    assert_eq!(repairs, 1);
    assert_eq!(messages.len(), 2);
    let merged = &messages[1];
    assert_eq!(merged["content"], json!("turn 1\nturn 2"));
    let ids: Vec<&str> = merged["tool_calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str))
        .collect();
    assert_eq!(ids, vec!["c1", "c2"]);
}

#[test]
fn repair_replaces_verification_candidate() {
    let mut messages = vec![
        json!({"role": "assistant", "content": "long candidate", "finish_reason": "verification_required"}),
        json!({"role": "assistant", "content": "verified reply", "finish_reason": "stop"}),
    ];
    let repairs = repair_message_sequence(&mut messages);
    assert_eq!(repairs, 1);
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], json!("verified reply"));
}

#[test]
fn repair_drops_stray_tool_and_merges_consecutive_users() {
    let mut messages = vec![
        json!({"role": "user", "content": "first"}),
        json!({"role": "user", "content": "second"}),
        json!({"role": "assistant", "content": "ok", "tool_calls": [{"id": "c1"}]}),
        json!({"role": "tool", "content": "result", "tool_call_id": "c1"}),
        json!({"role": "tool", "content": "orphan", "tool_call_id": "missing"}),
    ];
    let repairs = repair_message_sequence(&mut messages);
    assert_eq!(repairs, 2); // one user-merge + one stray tool drop
    let roles: Vec<&str> = messages.iter().filter_map(|m| m.get("role").and_then(Value::as_str)).collect();
    assert_eq!(roles, vec!["user", "assistant", "tool"]);
    assert_eq!(messages[0]["content"], json!("first\n\nsecond"));
}

#[test]
fn repair_codex_interim_turns_are_exempt_from_merge() {
    let mut messages = vec![
        json!({"role": "assistant", "content": "interim 1", "codex_reasoning_items": [{"x": 1}]}),
        json!({"role": "assistant", "content": "interim 2", "codex_message_items": [{"y": 2}]}),
    ];
    let repairs = repair_message_sequence(&mut messages);
    assert_eq!(repairs, 0);
    assert_eq!(messages.len(), 2);
}

#[test]
fn repair_invalidates_api_content_sidecar_on_user_merge() {
    let mut messages = vec![
        json!({"role": "user", "content": "a", "api_content": "A"}),
        json!({"role": "user", "content": "b", "api_content": "B"}),
    ];
    repair_message_sequence(&mut messages);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].get("api_content").is_none());
    assert_eq!(messages[0]["content"], json!("a\n\nb"));
}

// =====================================================================
// include_ancestors dedup + get_ancestor_display_prefix + lineage
// =====================================================================

#[test]
fn conversation_include_ancestors_dedups_replayed_user_message() {
    let (_dir, db) = open_db("state.db");
    // Root leaves its user turn as the last content row (the flush cursor
    // was reset at compression, so no assistant answer landed); the
    // continuation replays the same user text. The duplicate must be spliced
    // out of the lineage projection.
    create(&db, "root", "cli");
    append(&db, "root", "user", "hello");
    set_ended(&db, "root", 1_700_000_100.0, "compression");
    db.create_session("cont", "cli", &NewSession {
        parent_session_id: Some("root".into()),
        ..Default::default()
    })
    .expect("cont");
    append(&db, "cont", "user", "hello"); // replayed user turn
    append(&db, "cont", "assistant", "again");

    // Without ancestors: cont alone (both rows).
    let tip_only = db.get_messages_as_conversation("cont", false, false, false, false).expect("tip");
    assert_eq!(tip_only.len(), 2);
    // With ancestors: the replayed "hello" user message is deduped because
    // the same user content is adjacent earlier in the lineage.
    let lineage = db.get_messages_as_conversation("cont", true, false, false, false).expect("lineage");
    let users: Vec<&str> = lineage.iter().filter_map(|m| {
        if m.get("role").and_then(Value::as_str) == Some("user") {
            m.get("content").and_then(Value::as_str)
        } else {
            None
        }
    }).collect();
    assert_eq!(users, vec!["hello"]);
    // The assistant reply survives.
    assert!(lineage.iter().any(|m| m.get("content").and_then(Value::as_str) == Some("again")));
}

#[test]
fn ancestor_display_prefix_isolates_non_tip_messages() {
    let (_dir, db) = open_db("state.db");
    create(&db, "root", "cli");
    append(&db, "root", "user", "old question");
    append(&db, "root", "assistant", "old answer");
    set_ended(&db, "root", 1_700_000_100.0, "compression");
    db.create_session("tip", "cli", &NewSession {
        parent_session_id: Some("root".into()),
        ..Default::default()
    })
    .expect("tip");
    append(&db, "tip", "user", "new question");
    append(&db, "tip", "assistant", "new answer");

    let prefix = db.get_ancestor_display_prefix("tip").expect("prefix");
    assert_eq!(prefix.len(), 2);
    assert!(prefix.iter().all(|m| m.get("content").and_then(Value::as_str).map(|c| c.starts_with("old")).unwrap_or(false)));
    // Single-session lineage -> empty prefix.
    assert!(db.get_ancestor_display_prefix("root").expect("root prefix").is_empty());
}

// =====================================================================
// restore_rewound
// =====================================================================

#[test]
fn restore_rewound_flips_inactive_rows_back() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "one");
    append(&db, "s1", "assistant", "two");
    append(&db, "s1", "user", "three");
    let rows: Vec<i64> = {
        let conn = db.writer_conn();
        let mut stmt = conn.prepare("SELECT id FROM messages WHERE session_id = 's1' ORDER BY id").expect("stmt");
        stmt.query_map([], |r| r.get(0)).expect("map").collect::<Result<_, _>>().expect("collect")
    };
    // rewind to the final user row (id = rows[2]) soft-deletes rows[2].
    let result = db.rewind_to_message("s1", rows[2]).expect("rewind");
    assert_eq!(result.rewound_count, 1);

    // restore from rows[1] flips the inactive row back.
    let restored = db.restore_rewound("s1", rows[1]).expect("restore");
    assert_eq!(restored, 1);
    let conv = db.get_messages_as_conversation("s1", false, false, false, false).expect("conv");
    assert_eq!(conv.len(), 3);

    // Re-restoring is a no-op (0 rows flipped).
    assert_eq!(db.restore_rewound("s1", rows[0]).expect("restore again"), 0);
}

#[test]
fn conversation_observed_and_platform_message_id_surface() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "telegram");
    db.append_message("s1", &MessageInput {
        role: "user".into(),
        content: Some(json!("ping")),
        platform_message_id: Some("tg-99".into()),
        ..Default::default()
    }, None).expect("append");

    let conv = db.get_messages_as_conversation("s1", false, false, false, false).expect("conv");
    assert_eq!(conv[0]["message_id"].as_str(), Some("tg-99"));
}
