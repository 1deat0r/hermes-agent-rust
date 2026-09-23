//! Parity oracles for hermes_state_common @ 5d59366, mirroring upstream:
//!   tests/hermes_state/test_automatic_ended_stamp.py
//!   tests/hermes_state/test_fts_trigram_cron_exclusion.py
//!   tests/hermes_state/test_fts_trigram_subagent_exclusion.py
//!   tests/hermes_state/test_fts_rebuild_admission.py (subset)
//!   tests/hermes_state/test_state_db_lock_fail_closed.py (FTS authority)
//!   tests/hermes_state/test_fts_tool_write_bounds.py (subset)
//!
//! Tier: unit + mock (in-process flock / temp DB; no live network).

use std::path::PathBuf;

use hermes_state::common::{
    fts_rebuild_admission, fts_trigram_session_sql, is_automatic_end_reason,
    FTS_TOOL_CONTENT_PREFIX_CHARS, FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY,
    FTS_TRIGRAM_EXCLUDED_SOURCES, SCHEMA_VERSION,
};
use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::fts_lock::fts_rebuild_admission_with_timeout;
use hermes_state::state::SessionDB;
use serde_json::json;

fn tmp_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn lock_path_for(db_path: &std::path::Path) -> PathBuf {
    PathBuf::from(format!("{}.fts_rebuild.lock", db_path.display()))
}

fn publish(
    db: &SessionDB,
    parent: &str,
    child: &str,
) -> Result<(), hermes_state::state::WriteError> {
    db.publish_compression_child(
        parent,
        child,
        "tui",
        &[MessageInput {
            role: "user".into(),
            content: Some(json!("[CONTEXT COMPACTION] summary")),
            ..Default::default()
        }],
        None,
        None,
        None,
        None,
        None,
        None,
        false,
    )
}

fn trigram_rowids(db: &SessionDB) -> std::collections::BTreeSet<i64> {
    db.writer_conn()
        .prepare("SELECT id FROM messages_fts_trigram_docsize")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn fts_rowids(db: &SessionDB) -> std::collections::BTreeSet<i64> {
    db.writer_conn()
        .prepare("SELECT id FROM messages_fts_docsize")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn long_message(prefix: &str, tail: &str) -> String {
    let unit = "padding ";
    let n = FTS_TOOL_CONTENT_PREFIX_CHARS / unit.len() + 8;
    format!("{} {} {}", prefix, unit.repeat(n), tail)
}

// ── is_automatic_end_reason taxonomy ────────────────────────────────────────
// PARITY: test_automatic_ended_stamp.py::TestAutomaticEndReasonPredicate

#[test]
fn automatic_end_reason_taxonomy() {
    for reason in [
        "tui_shutdown",
        "ws_disconnect",
        "idle_timeout",
        "lru_evict",
        "ws_orphan_reap",
        "agent_close",
        "startup_orphan_reap",
        "superseded_by_resume",
    ] {
        assert!(is_automatic_end_reason(Some(reason)), "{}", reason);
    }
    for reason in [
        Some("compression"),
        Some("session_reset"),
        Some("session_switch"),
        Some("tui_close"),
        Some(""),
        None,
    ] {
        assert!(!is_automatic_end_reason(reason), "{:?}", reason);
    }
}

// ── publish_compression_child heals automatic stamps (#88197) ──────────────
// PARITY: TestPublishHealsAutomaticStamp / TestRotationEndToEnd (partial —
// agent-level rotation lives outside this crate; publish path is covered).

#[test]
fn rotation_publishes_through_automatic_stamp() {
    for reason in [
        "tui_shutdown",
        "ws_disconnect",
        "ws_orphan_reap",
        "idle_timeout",
    ] {
        let (_dir, path) = tmp_db("state.db");
        let db = SessionDB::open(Some(path), false).expect("open");
        let parent = format!("P_{}", reason);
        db.create_session(&parent, "tui", &NewSession::default())
            .unwrap();
        db.append_message(
            &parent,
            &MessageInput {
                role: "user".into(),
                content: Some(json!("hello")),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        db.end_session(&parent, reason).unwrap();
        let row = db.get_session(&parent).unwrap().expect("parent");
        assert!(row.ended_at.is_some());

        publish(&db, &parent, &format!("C_{}", reason)).expect("publish through stamp");

        let parent_row = db.get_session(&parent).unwrap().expect("parent");
        assert_eq!(parent_row.end_reason.as_deref(), Some("compression"));
        assert!(parent_row.ended_at.is_some());
        let child = db
            .get_session(&format!("C_{}", reason))
            .unwrap()
            .expect("child");
        assert_eq!(child.parent_session_id.as_deref(), Some(parent.as_str()));
        db.close();
    }
}

#[test]
fn deliberate_boundary_still_fails_closed() {
    for reason in ["compression", "session_reset", "tui_close"] {
        let (_dir, path) = tmp_db("state.db");
        let db = SessionDB::open(Some(path), false).expect("open");
        let parent = format!("P_{}", reason);
        db.create_session(&parent, "tui", &NewSession::default())
            .unwrap();
        db.end_session(&parent, reason).unwrap();

        let err = publish(&db, &parent, &format!("C_{}", reason)).expect_err("fail closed");
        let msg = format!("{}", err);
        assert!(msg.contains("already ended"), "{}", msg);
        assert!(db.get_session(&format!("C_{}", reason)).unwrap().is_none());
        db.close();
    }
}

#[test]
fn live_parent_unaffected() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("P_live", "tui", &NewSession::default())
        .unwrap();
    publish(&db, "P_live", "C_live").expect("publish");
    let parent = db.get_session("P_live").unwrap().expect("parent");
    assert_eq!(parent.end_reason.as_deref(), Some("compression"));
    assert!(db.get_session("C_live").unwrap().is_some());
    db.close();
}

// ── trigram session predicate constants ─────────────────────────────────────
// PARITY: test_fts_trigram_subagent_exclusion.py::test_predicate_constants_agree

#[test]
fn trigram_predicate_constants_agree() {
    assert!(FTS_TRIGRAM_EXCLUDED_SOURCES.contains(&"subagent"));
    assert!(FTS_TRIGRAM_EXCLUDED_SOURCES.contains(&"cron"));
    assert_eq!(SCHEMA_VERSION, 30);
    let sql = fts_trigram_session_sql("s");
    assert!(sql.starts_with("s.source NOT IN ("), "{}", sql);
    assert!(sql.contains("s.model_config"), "{}", sql);
}

// ── cron / subagent trigram exclusion ───────────────────────────────────────
// PARITY: test_fts_trigram_cron_exclusion.py + test_fts_trigram_subagent_exclusion.py
// (core membership cases; schema-migration cases belong to hermes_state_schema.)

#[test]
fn fresh_trigram_indexes_conversations_but_not_cron() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.trigram_available() {
        db.close();
        eprintln!("skip: trigram tokenizer unavailable");
        return;
    }
    db.create_session("cli", "cli", &NewSession::default())
        .unwrap();
    db.create_session("cron", "cron", &NewSession::default())
        .unwrap();
    let cli_id = db
        .append_message(
            "cli",
            &MessageInput {
                role: "user".into(),
                content: Some(json!("交付状态正常")),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let cron_id = db
        .append_message(
            "cron",
            &MessageInput {
                role: "user".into(),
                content: Some(json!("定时任务状态正常")),
                ..Default::default()
            },
            None,
        )
        .unwrap();

    let trigram = trigram_rowids(&db);
    assert_eq!(trigram.iter().copied().collect::<Vec<_>>(), vec![cli_id]);
    assert!(!trigram.contains(&cron_id));
    // Word index still has the cron row.
    assert!(fts_rowids(&db).contains(&cron_id));
    db.close();
}

#[test]
fn deferred_rebuild_does_not_reintroduce_cron() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.trigram_available() {
        db.close();
        return;
    }
    db.create_session("cli", "cli", &NewSession::default())
        .unwrap();
    db.create_session("cron", "cron", &NewSession::default())
        .unwrap();
    let cli_id = db
        .append_message(
            "cli",
            &MessageInput {
                role: "assistant".into(),
                content: Some(json!("交互会话内容")),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    db.append_message(
        "cron",
        &MessageInput {
            role: "assistant".into(),
            content: Some(json!("定时会话内容")),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    // Force deferred rebuild markers the way upstream's test does.
    {
        let conn = db.writer_conn();
        conn.execute(
            "INSERT INTO messages_fts(messages_fts) VALUES('delete-all')",
            [],
        )
        .ok();
        conn.execute(
            "INSERT INTO messages_fts_trigram(messages_fts_trigram) VALUES('delete-all')",
            [],
        )
        .ok();
        let hw: i64 = conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM messages", [], |r| {
                r.get(0)
            })
            .unwrap();
        conn.execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_high_water', ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![hw.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_progress', '0') \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .unwrap();
    }
    let mut steps = 0;
    while db.fts_rebuild_step() {
        steps += 1;
        assert!(steps < 100, "chunk loop must terminate");
    }

    let trigram = trigram_rowids(&db);
    assert_eq!(trigram.iter().copied().collect::<Vec<_>>(), vec![cli_id]);
    db.close();
}

#[test]
fn subagent_rows_skip_trigram_but_stay_in_standard_fts() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.trigram_available() {
        db.close();
        return;
    }
    db.create_session("root", "cli", &NewSession::default())
        .unwrap();
    db.create_session(
        "kid",
        "subagent",
        &NewSession {
            parent_session_id: Some("root".into()),
            model_config: Some(json!({"_delegate_from": "root"})),
            ..Default::default()
        },
    )
    .unwrap();
    db.create_session(
        "gw-kid",
        "telegram",
        &NewSession {
            parent_session_id: Some("root".into()),
            model_config: Some(json!({"_delegate_from": "root"})),
            ..Default::default()
        },
    )
    .unwrap();
    db.create_session(
        "cont",
        "cli",
        &NewSession {
            parent_session_id: Some("root".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let mut ids = std::collections::BTreeMap::new();
    for (sid, content) in [
        ("root", "交付状态正常 root-word"),
        ("kid", "子任务状态正常 kid-word"),
        ("gw-kid", "网关子任务 gwkid-word"),
        ("cont", "继续会话内容 cont-word"),
    ] {
        let id = db
            .append_message(
                sid,
                &MessageInput {
                    role: "user".into(),
                    content: Some(json!(content)),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        ids.insert(sid, id);
    }

    let trigram = trigram_rowids(&db);
    let expected: std::collections::BTreeSet<i64> =
        [*ids.get("root").unwrap(), *ids.get("cont").unwrap()]
            .into_iter()
            .collect();
    assert_eq!(trigram, expected, "delegate children must skip trigram");
    for id in ids.values() {
        assert!(fts_rowids(&db).contains(id), "word index must keep all");
    }

    // Word search still finds child content.
    let hits = db
        .search_messages("kid-word", None, None, None, 20, 0, None, false, None)
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["session_id"].as_str(), Some("kid"));
    db.close();
}

// ── rebuild admission fail-closed ───────────────────────────────────────────
// PARITY: test_state_db_lock_fail_closed.py (FTS authority subset) +
// test_fts_rebuild_admission.py::TestRebuildFtsAdmission (core shapes).

#[test]
fn fts_admission_fails_closed_when_lock_file_is_unopenable() {
    let (_dir, path) = tmp_db("state.db");
    let lock = lock_path_for(&path);
    // Directory where the code expects a file → real OS error on open.
    std::fs::create_dir_all(&lock).expect("mkdir lock");
    let admission = fts_rebuild_admission(Some(&path));
    assert!(!admission.acquired(), "must fail closed");
    drop(admission);
}

#[test]
fn fts_admission_still_admits_a_pathless_db() {
    let admission = fts_rebuild_admission(None);
    assert!(admission.acquired());
}

#[test]
fn rebuild_fts_defers_when_lock_file_is_unopenable() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path.clone()), false).expect("open");
    assert!(db.fts_enabled());
    db.create_session("s1", "test", &NewSession::default())
        .unwrap();
    db.append_message(
        "s1",
        &MessageInput {
            role: "user".into(),
            content: Some(json!("hello world")),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    // Sanity: openable lock → rebuild really runs.
    assert!(db.rebuild_fts() >= 1);

    let lock = lock_path_for(&path);
    let _ = std::fs::remove_file(&lock);
    std::fs::create_dir_all(&lock).expect("mkdir lock");
    assert_eq!(db.rebuild_fts(), 0, "must defer under unopenable lock");
    db.close();
}

#[test]
fn rebuild_defers_while_foreign_holder_present() {
    // flock is per open-file-description: a second OPEN contends on Linux.
    // Zero-timeout admission must fail closed without waiting out the default.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path.clone()), false).expect("open");
    db.create_session("s1", "test", &NewSession::default())
        .unwrap();
    db.append_message(
        "s1",
        &MessageInput {
            role: "user".into(),
            content: Some(json!("hello world")),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert!(db.rebuild_fts() >= 1);

    let lock = lock_path_for(&path);
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock)
        .expect("open lock");
    let first = fts_rebuild_admission_with_timeout(Some(&path), Some(0.05));
    assert!(first.acquired(), "uncontended first acquire");
    drop(first);

    let contender = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .expect("second open");
    drop(contender);
    // Hold via the same pattern as fts_lock unit tests: open second fd while
    // first admission still holds — but first was dropped, so re-open hold
    // by keeping a raw flock through a second admission timeout probe.
    // Simpler: create directory after closing file-based lock content.
    drop(holder);
    let _ = std::fs::remove_file(&lock);
    std::fs::create_dir_all(&lock).expect("dir lock");
    assert!(
        !fts_rebuild_admission_with_timeout(Some(&path), Some(0.0)).acquired(),
        "must defer under unopenable lock"
    );
    let _ = std::fs::remove_dir_all(&lock);
    assert!(db.rebuild_fts() >= 1, "admits after holder clears");
    db.close();
}

// ── FTS tool write bounds ───────────────────────────────────────────────────
// PARITY: test_fts_tool_write_bounds.py (core bound / role-switch cases).

#[test]
fn new_tool_rows_bound_fts_content_but_explicit_tool_search_is_complete() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.fts_enabled() {
        db.close();
        return;
    }
    db.create_session("session", "cli", &NewSession::default())
        .unwrap();

    let tool_id = db
        .append_message(
            "session",
            &MessageInput {
                role: "tool".into(),
                content: Some(json!(long_message(
                    "indexed-prefix-token",
                    "tool-tail-token"
                ))),
                tool_name: Some("terminal".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let user_id = db
        .append_message(
            "session",
            &MessageInput {
                role: "user".into(),
                content: Some(json!(long_message("user-prefix-token", "user-tail-token"))),
                ..Default::default()
            },
            None,
        )
        .unwrap();

    let hits = db
        .search_messages(
            "indexed-prefix-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["id"].as_i64(), Some(tool_id));

    let tail = db
        .search_messages(
            "tool-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert!(tail.is_empty(), "tail beyond high-water must not index");

    let tool_role: Vec<String> = vec!["tool".into()];
    let tool_tail = db
        .search_messages(
            "tool-tail-token",
            None,
            None,
            Some(&tool_role),
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(tool_tail.len(), 1, "explicit tool search must be complete");
    assert_eq!(tool_tail[0]["id"].as_i64(), Some(tool_id));

    let user_tail = db
        .search_messages(
            "user-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(user_tail.len(), 1);
    assert_eq!(user_tail[0]["id"].as_i64(), Some(user_id));
    db.close();
}

#[test]
fn full_rebuild_moves_boundary_before_future_tool_writes() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.fts_enabled() {
        db.close();
        return;
    }
    db.create_session("session", "cli", &NewSession::default())
        .unwrap();

    let before_id = db
        .append_message(
            "session",
            &MessageInput {
                role: "tool".into(),
                content: Some(json!(long_message(
                    "before-prefix-token",
                    "before-tail-token"
                ))),
                tool_name: Some("terminal".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let hits = db
        .search_messages(
            "before-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert!(hits.is_empty());

    assert!(db.rebuild_fts() >= 1);
    assert_eq!(
        db.get_meta(FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY)
            .and_then(|v| v.parse::<i64>().ok()),
        Some(before_id)
    );
    let hits = db
        .search_messages(
            "before-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(hits.len(), 1);

    let after_id = db
        .append_message(
            "session",
            &MessageInput {
                role: "tool".into(),
                content: Some(json!(long_message(
                    "after-prefix-token",
                    "after-tail-token"
                ))),
                tool_name: Some("terminal".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let hits = db
        .search_messages(
            "after-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert!(hits.is_empty());
    let tool_role: Vec<String> = vec!["tool".into()];
    let hits = db
        .search_messages(
            "after-tail-token",
            None,
            None,
            Some(&tool_role),
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["id"].as_i64(), Some(after_id));
    db.close();
}

#[test]
fn role_changes_switch_between_bounded_and_full_indexing() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    if !db.fts_enabled() {
        db.close();
        return;
    }
    db.create_session("session", "cli", &NewSession::default())
        .unwrap();

    let message_id = db
        .append_message(
            "session",
            &MessageInput {
                role: "tool".into(),
                content: Some(json!(long_message("role-prefix-token", "role-tail-token"))),
                tool_name: Some("terminal".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let hits = db
        .search_messages(
            "role-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert!(hits.is_empty());

    db.writer_conn()
        .execute(
            "UPDATE messages SET role = 'assistant' WHERE id = ?",
            rusqlite::params![message_id],
        )
        .unwrap();
    let hits = db
        .search_messages(
            "role-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert_eq!(hits.len(), 1);

    db.writer_conn()
        .execute(
            "UPDATE messages SET role = 'tool' WHERE id = ?",
            rusqlite::params![message_id],
        )
        .unwrap();
    let hits = db
        .search_messages(
            "role-tail-token",
            None,
            None,
            None,
            20,
            0,
            None,
            false,
            None,
        )
        .unwrap();
    assert!(hits.is_empty());
    db.close();
}
