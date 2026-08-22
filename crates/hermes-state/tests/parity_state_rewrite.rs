//! Parity oracles for the message rewrite / rewind / in-place-compaction
//! surfaces, mirroring upstream tests @ b9aa928:
//!   tests/test_hermes_state.py (replace_messages timestamp/metadata pairs)
//!   tests/hermes_state/test_replace_messages_archive_siblings.py
//!   tests/gateway/test_undo_rewind_session.py (DB-level contract)

use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::rewrite::RewindOutcome;
use hermes_state::state::{SessionDB, WriteError};
use serde_json::{json, Value};

fn tmp_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn msg(role: &str, content: &str) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(json!(content)),
        ..Default::default()
    }
}

fn msg_ts(role: &str, content: &str, ts: f64) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(json!(content)),
        timestamp: Some(ts),
        ..Default::default()
    }
}

fn live_contents(db: &SessionDB, sid: &str) -> Vec<String> {
    db.get_messages(sid, false, None, 0)
        .unwrap()
        .into_iter()
        .filter_map(|m| match m.content {
            Some(Value::String(s)) => Some(s),
            _ => None,
        })
        .collect()
}

fn all_rows(db: &SessionDB, sid: &str) -> Vec<(String, String, bool, bool)> {
    // (content, role, active, compacted)
    let rows: Vec<(String, String, bool, bool)> = db.get_messages(sid, true, None, 0)
        .unwrap()
        .into_iter()
        .map(|m| {
            (
                m.content.and_then(|c| c.as_str().map(str::to_string)).unwrap_or_default(),
                m.role.clone(),
                m.active,
                m.compacted,
            )
        })
        .collect();
    rows
}

#[test]
fn replace_messages_preserves_explicit_timestamps() {
    // test_replace_messages_preserves_timestamps
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    let msgs = [
        msg_ts("user", "first", 100.0),
        msg_ts("assistant", "second", 200.0),
        msg_ts("user", "third", 300.0),
    ];
    db.replace_messages("s1", &msgs, false).unwrap();
    let out = db.get_messages("s1", false, None, 0).unwrap();
    assert_eq!(out.len(), 3);
    let ts: Vec<f64> = out.iter().map(|m| m.timestamp).collect();
    assert_eq!(ts, vec![100.0, 200.0, 300.0]);
    // Raw stored timestamps match too.
    let raw: Vec<f64> = db
        .writer_conn()
        .prepare("SELECT timestamp FROM messages WHERE session_id = 's1' ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(raw, vec![100.0, 200.0, 300.0]);
    db.close();
}

#[test]
fn compression_replace_roundtrip_preserves_timestamps() {
    // test_compression_replace_roundtrip_preserves_timestamps (#28841)
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let timestamps = [1_500_000_000.0, 1_500_000_100.0, 1_500_000_200.0];
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    for (i, ts) in timestamps.iter().enumerate() {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        db.append_message("s1", &msg_ts(role, &format!("msg-{}", i), *ts), None)
            .unwrap();
    }
    // Compression keeps the last two turns verbatim, prepends a summary that
    // has NO timestamp (falls back to now — later than the fixed points).
    let compressed = vec![
        msg("user", "[summary]"),
        msg_ts("assistant", "msg-1", timestamps[1]),
        msg_ts("user", "msg-2", timestamps[2]),
    ];
    db.replace_messages("s1", &compressed, false).unwrap();
    let raw: Vec<f64> = db
        .writer_conn()
        .prepare("SELECT timestamp FROM messages WHERE session_id = 's1' ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(raw.len(), 3);
    assert_eq!(raw[1..], timestamps[1..]);
    assert!(raw[0] > timestamps[2]); // summary stamped with a current time
    db.close();
}

#[test]
fn replace_messages_preserves_display_metadata() {
    // test_replace_messages_preserves_display_metadata (round-trip via
    // _insert_message_rows)
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    let meta = json!({"kind": "note", "n": 3});
    let m = MessageInput {
        role: "assistant".to_string(),
        content: Some(json!("with meta")),
        display_metadata: Some(meta.clone()),
        ..Default::default()
    };
    db.replace_messages("s1", &[m], false).unwrap();
    let out = db.get_messages("s1", false, None, 0).unwrap();
    assert_eq!(out[0].display_metadata.as_ref(), Some(&meta));
    db.close();
}

fn seed_compacted_session(db: &SessionDB, sid: &str) {
    // _seed_compacted_session from test_replace_messages_archive_siblings
    db.create_session(sid, "test", &NewSession::default()).unwrap();
    db.append_messages_batch(
        sid,
        &[
            msg("user", "old question"),
            msg("assistant", "old answer"),
            msg("user", "another old question"),
            msg("assistant", "another old answer"),
        ],
        None,
        None,
    )
    .unwrap();
    db.archive_and_compact(
        sid,
        &[
            msg("assistant", "summary of old turns"),
            msg("user", "live question"),
            msg("assistant", "live answer"),
        ],
        None,
    )
    .unwrap();
}

fn archived_count(db: &SessionDB, sid: &str) -> usize {
    db.get_messages(sid, true, None, 0)
        .unwrap()
        .iter()
        .filter(|m| !m.active)
        .count()
}

#[test]
fn active_only_replace_preserves_archived_rows() {
    // test_persist_nonowned_branch_keeps_archived_rows (#80216 class)
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    seed_compacted_session(&db, "acp-compacted");
    assert_eq!(archived_count(&db, "acp-compacted"), 4);

    db.replace_messages(
        "acp-compacted",
        &[msg("user", "rewritten"), msg("assistant", "rewritten answer")],
        true,
    )
    .unwrap();

    assert_eq!(archived_count(&db, "acp-compacted"), 4);
    assert_eq!(
        live_contents(&db, "acp-compacted"),
        vec!["rewritten", "rewritten answer"]
    );
    // Counts track the LIVE set only.
    let row = db.get_session("acp-compacted").unwrap().unwrap();
    assert_eq!(row.message_count, 2);
    db.close();
}

#[test]
fn fresh_session_active_only_equals_full_replace() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "acp-fresh";
    db.create_session(sid, "test", &NewSession::default()).unwrap();
    db.append_messages_batch(sid, &[msg("user", "q"), msg("assistant", "a")], None, None)
        .unwrap();
    db.replace_messages(sid, &[msg("user", "only")], true).unwrap();
    assert_eq!(live_contents(&db, sid), vec!["only"]);
    assert_eq!(archived_count(&db, sid), 0);
    db.close();
}

#[test]
fn destructive_replace_removes_archived_rows_too() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "dest";
    seed_compacted_session(&db, sid);
    db.replace_messages(sid, &[msg("user", "full rewrite")], false).unwrap();
    assert_eq!(archived_count(&db, sid), 0);
    assert_eq!(live_contents(&db, sid), vec!["full rewrite"]);
    db.close();
}

#[test]
fn replace_rejects_compression_closed_session() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "closed";
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    db.append_message(sid, &msg("user", "x"), None).unwrap();
    db.end_session(sid, "compression").unwrap();
    let err = db.replace_messages(sid, &[msg("user", "y")], false).unwrap_err();
    assert!(
        matches!(err, WriteError::CompressionSessionClosed(_)),
        "got {:?}",
        err
    );
    // Even the active-only rewrite path rejects the closed session.
    let err = db.replace_messages(sid, &[msg("user", "y")], true).unwrap_err();
    assert!(matches!(err, WriteError::CompressionSessionClosed(_)));
    db.close();
}

#[test]
fn has_archived_messages_probe() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "probe";
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    assert!(!db.has_archived_messages(sid).unwrap());
    db.append_message(sid, &msg("user", "q"), None).unwrap();
    db.archive_and_compact(sid, &[msg("assistant", "sum")], None).unwrap();
    assert!(db.has_archived_messages(sid).unwrap());
    db.close();
}

#[test]
fn archive_and_compact_archives_and_counts_active_set() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "ac";
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    db.append_messages_batch(
        sid,
        &[
            msg("user", "q1"),
            msg("assistant", "a1"),
            msg("user", "q2"),
            msg("assistant", "a2"),
        ],
        None,
        None,
    )
    .unwrap();

    let inserted = db
        .archive_and_compact(
            sid,
            &[msg("assistant", "summary"), msg("user", "q3")],
            None,
        )
        .unwrap();
    assert_eq!(inserted, 2);

    let rows = all_rows(&db, sid);
    // 4 archived (active=0, compacted=1) + 2 live rows.
    let archived: Vec<_> = rows.iter().filter(|r| !r.2).collect();
    let live: Vec<_> = rows.iter().filter(|r| r.2).collect();
    assert_eq!(archived.len(), 4);
    assert!(archived.iter().all(|r| r.3));
    assert_eq!(live.len(), 2);
    assert!(live.iter().all(|r| !r.3));

    let row = db.get_session(sid).unwrap().unwrap();
    assert_eq!(row.message_count, 2);
    db.close();
}

#[test]
fn archive_and_compact_merges_model_config_patch() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "cfg";
    db.create_session(sid, "cli", &NewSession {
        model_config: Some(json!({"a": 1, "keep": "yes", "remove": "me"})),
        ..Default::default()
    })
    .unwrap();
    let mut patch = serde_json::Map::new();
    patch.insert("b".to_string(), json!(2));
    patch.insert("remove".to_string(), Value::Null); // delete key
    db.archive_and_compact(sid, &[msg("assistant", "sum")], Some(&patch)).unwrap();

    let raw_config: Option<String> = db
        .writer_conn()
        .query_row("SELECT model_config FROM sessions WHERE id = ?", rusqlite::params![sid], |r| r.get(0))
        .unwrap();
    let cfg: Value = serde_json::from_str(&raw_config.unwrap()).unwrap();
    assert_eq!(cfg["a"], json!(1));
    assert_eq!(cfg["keep"], json!("yes"));
    assert_eq!(cfg["b"], json!(2));
    assert!(cfg.get("remove").is_none());

    // Patching a vanished session raises ValueError (on_missing="raise").
    let err = db.archive_and_compact("ghost", &[msg("assistant", "s")], Some(&patch)).unwrap_err();
    assert!(matches!(err, WriteError::ValueError(_)), "got {:?}", err);
    db.close();
}

// ── rewind_to_message ───────────────────────────────────────────────────────

fn seed_turns(db: &SessionDB, sid: &str, turns: usize) {
    db.create_session(sid, "telegram", &NewSession::default()).unwrap();
    for i in 1..=turns {
        db.append_message(sid, &msg("user", &format!("q{}", i)), None).unwrap();
        db.append_message(sid, &msg("assistant", &format!("a{}", i)), None).unwrap();
    }
}

#[test]
fn rewind_soft_deletes_from_target_to_tail() {
    // DB-level contract of the gateway /rewind flow (#21910): rewinding to
    // q3 removes q3 + a3 and leaves the pre-q3 head active.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "gw-1";
    seed_turns(&db, sid, 3);

    // Messages ids: 1=q1, 2=a1, 3=q2, 4=a2, 5=q3, 6=a3.
    let res: RewindOutcome = db.rewind_to_message(sid, 5).unwrap();
    assert_eq!(res.rewound_count, 2);
    assert_eq!(res.target_message["content"], json!("q3"));
    assert_eq!(res.target_message["role"], json!("user"));
    assert_eq!(res.new_head_id, Some(4));
    assert_eq!(
        live_contents(&db, sid),
        vec!["q1", "a1", "q2", "a2"]
    );
    // Rewound rows stay on disk (active=0) for audit.
    let inactive: Vec<_> = db.get_messages(sid, true, None, 0)
        .unwrap().into_iter().filter(|m| !m.active).collect();
    assert_eq!(inactive.len(), 2);
    // rewind_count is bumped.
    let count: i64 = db.writer_conn()
        .query_row("SELECT COALESCE(rewind_count, 0) FROM sessions WHERE id = ?", rusqlite::params![sid], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    db.close();
}

#[test]
fn rewind_two_turns_and_validation() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "gw-2";
    seed_turns(&db, sid, 3);

    // Rewind two turns (target q2 = id 3): q2,a2,q3,a3 all go inactive.
    let res = db.rewind_to_message(sid, 3).unwrap();
    assert_eq!(res.rewound_count, 4);
    assert_eq!(live_contents(&db, sid), vec!["q1", "a1"]);

    // Idempotent on the active flag: re-rewinding same target flips nothing
    // but still bumps the counter.
    let res = db.rewind_to_message(sid, 3).unwrap();
    assert_eq!(res.rewound_count, 0);
    assert_eq!(res.new_head_id, Some(2));

    // Target must be a user message: id 2 is the assistant "a1".
    let err = db.rewind_to_message(sid, 2).unwrap_err();
    assert!(matches!(err, WriteError::ValueError(_)), "got {:?}", err);
    // Missing target.
    let err = db.rewind_to_message(sid, 999).unwrap_err();
    assert!(matches!(err, WriteError::ValueError(_)), "got {:?}", err);
    // Wrong-session target.
    db.create_session("other", "cli", &NewSession::default()).unwrap();
    let err = db.rewind_to_message("other", 1).unwrap_err();
    assert!(matches!(err, WriteError::ValueError(_)), "got {:?}", err);
    db.close();
}

#[test]
fn rewind_to_latest_rewinds_to_empty_head() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "one";
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    db.append_message(sid, &msg("user", "only"), None).unwrap();
    let res = db.rewind_to_message(sid, 1).unwrap();
    assert_eq!(res.rewound_count, 1);
    assert_eq!(res.new_head_id, None);
    assert!(live_contents(&db, sid).is_empty());
    db.close();
}
