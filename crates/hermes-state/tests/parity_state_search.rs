//! Parity oracles for the SessionSearchMixin, mirroring upstream @ b9aa928:
//!   tests/test_hermes_state.py (TestFTS5Search, TestCJKSearchFallback,
//!     exclude-sources + tool-visibility regressions, sanitizer unit)
//!   tests/hermes_state/test_get_anchored_view.py
//!   tests/state/test_fts_runtime_rebuild.py (rebuild/optimize surfaces)

use std::collections::HashSet;
use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::search::{contains_cjk, sanitize_fts5_query};
use hermes_state::state::SessionDB;
use serde_json::json;

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

// ── TestFTS5Search ─────────────────────────────────────────────────────────

#[test]
fn search_finds_content_and_returns_context() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "How do I deploy with Docker?"), None).unwrap();
    db.append_message("s1", &msg("assistant", "Use docker compose up."), None).unwrap();

    let results = db.search_messages("docker", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 2);
    let snippets: Vec<&str> = results.iter().filter_map(|r| r.get("snippet").and_then(|v| v.as_str())).collect();
    assert!(snippets.iter().any(|s| s.to_lowercase().contains("docker")));

    // Context is present by default and non-empty.
    db.append_message("s1", &msg("user", "Tell me about Kubernetes"), None).unwrap();
    db.append_message("s1", &msg("assistant", "Kubernetes is an orchestrator."), None).unwrap();
    let results = db.search_messages("Kubernetes", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].get("context").is_some());
    assert!(results[0]["context"].as_array().map(|a| !a.is_empty()).unwrap_or(false));
    db.close();
}

#[test]
fn search_fields_project_results_without_changing_default() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "Tell me about Kubernetes"), None).unwrap();
    db.append_message("s1", &msg("assistant", "Kubernetes is an orchestrator."), None).unwrap();

    let fields = vec!["session_id".to_string(), "role".to_string(), "snippet".to_string()];
    let projected = db.search_messages("Kubernetes", None, None, None, 20, 0, None, false, Some(&fields)).unwrap();
    let default = db.search_messages("Kubernetes", None, None, None, 20, 0, None, false, None).unwrap();

    assert_eq!(projected.len(), default.len());
    assert!(projected.iter().all(|r| {
        let keys: HashSet<&str> = r.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        keys == ["session_id", "role", "snippet"].iter().copied().collect()
    }));
    assert!(default.iter().all(|r| r["context"].as_array().map(|a| !a.is_empty()).unwrap_or(false)));

    // Unknown field -> ValueError.
    let bad = vec!["nope".to_string()];
    let err = db.search_messages("Kubernetes", None, None, None, 20, 0, None, false, Some(&bad)).unwrap_err();
    assert!(matches!(err, hermes_state::state::WriteError::ValueError(_)), "got {:?}", err);
    db.close();
}

#[test]
fn sanitizer_matches_upstream_unit_cases() {
    // TestFTS5Search::test_sanitize_fts5_query_strips_dangerous_chars
    let s = sanitize_fts5_query;
    assert_eq!(s("hello world"), "hello world");
    assert!(!s("C++").contains('+'));
    assert!(!s("\"unterminated").contains('"'));
    assert!(!s("(problem").contains('('));
    assert!(!s("{test}").contains('{'));
    assert_eq!(s("hello AND"), "hello");
    assert_eq!(s("OR world"), "world");
    assert_eq!(s("***"), "");
    assert_eq!(s("deploy*"), "deploy*");
    assert!(!s("TODO: fix").contains(':'));
    assert_eq!(s("TODO: fix").split_whitespace().collect::<Vec<_>>(), vec!["TODO", "fix"]);
    assert!(!s("error:timeout").contains(':'));
    // Dotted/hyphenated terms are quoted (exact phrase semantics).
    assert_eq!(s("chat-send"), "\"chat-send\"");
}

#[test]
fn long_search_query_is_capped_and_does_not_crash() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "bounded sanitizer target"), None).unwrap();

    let query = format!("{}{}", "\"".repeat(50_000), " bounded".repeat(10_000));
    let start = std::time::Instant::now();
    let _results = db.search_messages(&query, None, None, None, 20, 0, None, false, None).unwrap();
    // Vec<Value> (not a JSON array) is the Rust surface; nothing to assert beyond it returned.
    assert!(start.elapsed().as_secs_f64() < 10.0, "sanitizer must not blow up");
    db.close();
}

// ── TestCJKSearchFallback ──────────────────────────────────────────────────

#[test]
fn cjk_detection_covers_all_ranges() {
    assert!(contains_cjk("记忆断裂"));
    assert!(contains_cjk("こんにちは"));
    assert!(contains_cjk("カタカナ"));
    assert!(contains_cjk("안녕하세요"));
    assert!(contains_cjk("기억"));
    assert!(!contains_cjk("hello world"));
    assert!(contains_cjk("日本語mixedwithenglish"));
    assert!(!contains_cjk(""));
}

#[test]
fn mixed_cjk_english_query_and_like_wildcards() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "讨论Agent通信协议"), None).unwrap();
    let results = db.search_messages("Agent通信", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 1);

    // % in a CJK query must be literal (only matches s1's row).
    db.create_session("s2", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "达成100%完成率"), None).unwrap();
    db.append_message("s2", &msg("user", "达成100完成率是目标"), None).unwrap();
    let results = db.search_messages("100%完成", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["session_id"], json!("s1"));
    db.close();
}

// ── filters + tool visibility ──────────────────────────────────────────────

#[test]
fn search_messages_excludes_tool_source() {
    // tests/test_hermes_state.py::test_search_messages_excludes_tool_source
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("py", "cli", &NewSession::default()).unwrap();
    db.create_session("tool", "tool", &NewSession::default()).unwrap();
    db.append_message("py", &msg("user", "Python is a language used in infra"), None).unwrap();
    db.append_message("tool", &msg("user", "Python tool run details"), None).unwrap();

    let all = db.search_messages("Python", None, None, None, 20, 0, None, false, None).unwrap();
    assert!(all.iter().any(|r| r["session_id"] == json!("py")));
    let excluded = db.search_messages("Python", None, Some(&["tool".to_string()]), None, 20, 0, None, false, None).unwrap();
    assert!(excluded.iter().all(|r| r["session_id"] != json!("tool")));
    let only = db.search_messages("Python", Some(&["tool".to_string()]), None, None, 20, 0, None, false, None).unwrap();
    assert!(only.iter().any(|r| r["session_id"] == json!("tool")));
    db.close();
}

#[test]
fn search_messages_sees_tool_name_and_tool_calls() {
    // tool_name/tool_calls are FTS-indexed columns (#16751).
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message(
        "s1",
        &MessageInput {
            role: "tool".to_string(),
            content: Some(json!("plain")),
            tool_name: Some("UNIQUETOOLNAME".to_string()),
            tool_calls: Some(json!([{"name": "UNIQUESEARCHTOKEN"}])),
            ..Default::default()
        },
        None,
    ).unwrap();

    let r1 = db.search_messages("UNIQUETOOLNAME", None, None, None, 20, 0, None, false, None).unwrap();
    let r2 = db.search_messages("UNIQUESEARCHTOKEN", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(r1.len(), 1, "tool_name must be searchable: {:?}", r1);
    assert_eq!(r2.len(), 1, "tool_calls must be searchable: {:?}", r2);
    db.close();
}

#[test]
fn search_visibility_compacted_discoverable_rewound_hidden() {
    // archive_and_compact rows (active=0/compacted=1) stay discoverable by
    // default; rewind rows (active=0/compacted=0) drop out (#38763).
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_messages_batch("s1", &[msg("user", "needle alpha"), msg("assistant", "reply alpha")], None, None).unwrap();
    db.archive_and_compact("s1", &[msg("user", "needle beta")], None).unwrap();

    // Compacted-archived "needle alpha" rows ARE discoverable by default.
    let archived = db.search_messages("needle", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(archived.len(), 2); // alpha user + live beta (assistant has no needle)

    // Rewind the live head (the compacted summary row): it becomes
    // active=0/compacted=0 and drops out of default search.
    let target: i64 = db.writer_conn()
        .query_row("SELECT id FROM messages WHERE session_id='s1' AND active=1 LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let r = db.rewind_to_message("s1", target).unwrap();
    assert_eq!(r.rewound_count, 1);
    let after = db.search_messages("needle", None, None, None, 20, 0, None, false, None).unwrap();
    assert!(after.iter().all(|m| m["id"].as_i64() != Some(target)), "rewound live row must be hidden");
    // include_inactive=True finds it again.
    let all = db.search_messages("needle", None, None, None, 20, 0, None, true, None).unwrap();
    assert!(all.iter().any(|m| m["id"].as_i64() == Some(target)));
    db.close();
}

// ── get_anchored_view ──────────────────────────────────────────────────────

fn seed_long_session(db: &SessionDB, sid: &str, n: i64) -> Vec<i64> {
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    let mut ids = Vec::new();
    for i in 0..n {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let mid = db.append_message(sid, &msg(role, &format!("prose msg {}", i)), None).unwrap();
        ids.push(mid);
    }
    ids
}

#[test]
fn anchored_view_window_and_bookends() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let ids = seed_long_session(&db, "s1", 30);

    let view = db.get_anchored_view("s1", ids[15], 3, 3, None).unwrap();
    assert_eq!(view["window"].as_array().unwrap().len(), 7);
    assert_eq!(view["bookend_start"].as_array().unwrap().len(), 3);
    assert_eq!(view["bookend_end"].as_array().unwrap().len(), 3);
    let start_ids: Vec<i64> = view["bookend_start"].as_array().unwrap().iter().map(|m| m["id"].as_i64().unwrap()).collect();
    assert_eq!(start_ids, ids[..3]);
    let end_ids: Vec<i64> = view["bookend_end"].as_array().unwrap().iter().map(|m| m["id"].as_i64().unwrap()).collect();
    assert_eq!(end_ids, ids[27..]);

    // Empty window for a missing anchor.
    let view = db.get_anchored_view("s1", 9999, 3, 3, None).unwrap();
    assert!(view["window"].as_array().unwrap().is_empty());
    assert_eq!(view["messages_before"], json!(0));
    assert_eq!(view["messages_after"], json!(0));
    db.close();
}

#[test]
fn anchored_view_role_filter_and_anchor_preserved() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    let mut user_ids = Vec::new();
    for i in 0..5 {
        user_ids.push(db.append_message("s1", &msg("user", &format!("u{}", i)), None).unwrap());
        db.append_message(
            "s1",
            &MessageInput {
                role: "tool".to_string(),
                content: Some(json!(format!("tool output {}", i))),
                tool_name: Some("x".to_string()),
                ..Default::default()
            },
            None,
        ).unwrap();
    }
    let tool_id = db.writer_conn()
        .query_row("SELECT id FROM messages WHERE role='tool' AND session_id='s1' LIMIT 1", [], |r| r.get::<_, i64>(0))
        .unwrap();

    let roles = vec!["user".to_string(), "assistant".to_string()];
    let view = db.get_anchored_view("s1", user_ids[2], 5, 0, Some(&roles)).unwrap();
    let window_roles: Vec<&str> = view["window"].as_array().unwrap().iter().map(|m| m["role"].as_str().unwrap()).collect();
    assert!(!window_roles.contains(&"tool"));

    // Anchor on the tool message — preserved despite the default filter.
    let view = db.get_anchored_view("s1", tool_id, 5, 0, Some(&roles)).unwrap();
    let window_ids: Vec<i64> = view["window"].as_array().unwrap().iter().map(|m| m["id"].as_i64().unwrap()).collect();
    assert!(window_ids.contains(&tool_id));
    db.close();
}

// ── list_recent_user_messages ──────────────────────────────────────────────

#[test]
fn recent_user_messages_excludes_bookkeeping_and_handoffs() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "recent";
    db.create_session(sid, "cli", &NewSession::default()).unwrap();
    db.append_message(sid, &msg("user", "real turn one"), None).unwrap();
    db.append_message(sid, &msg("assistant", "a1"), None).unwrap();
    // Timeline bookkeeping row: display_kind set.
    db.append_message(
        sid,
        &MessageInput {
            role: "user".to_string(),
            content: Some(json!("model_switch marker")),
            display_kind: Some("model_switch".to_string()),
            ..Default::default()
        },
        None,
    ).unwrap();
    db.append_message(sid, &msg("user", "real turn two with a much longer payload that extends beyond eighty characters in total to exercise truncation behavior"), None).unwrap();

    let recent = db.list_recent_user_messages(sid, 20, false).unwrap();
    assert_eq!(recent.len(), 2);
    assert!(recent.iter().all(|r| r["preview"].as_str().unwrap() != "model_switch marker"));
    assert_eq!(recent[0]["preview"].as_str().unwrap().chars().count(), 80);
    assert_eq!(recent[0]["preview"].as_str().unwrap().chars().last(), Some('.'));

    // Compaction handoff (SUMMARY_PREFIX) is never a user-originated turn.
    db.append_message(
        sid,
        &MessageInput {
            role: "user".to_string(),
            content: Some(json!(hermes_state::compression_prefix::SUMMARY_PREFIX)),
            ..Default::default()
        },
        None,
    ).unwrap();
    let recent2 = db.list_recent_user_messages(sid, 20, false).unwrap();
    assert!(recent2.iter().all(|r| !r["preview"].as_str().unwrap().starts_with("[CONTEXT COMPACTION")));
    db.close();
}

// ── rebuild / optimize surfaces ────────────────────────────────────────────

#[test]
fn optimize_and_rebuild_are_safe_on_fresh_db() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    db.append_message("s1", &msg("user", "needle in the haystack"), None).unwrap();

    // rebuild_fts / optimize_fts touch every present index.
    let rebuilt = db.rebuild_fts();
    assert!(rebuilt >= 1, "rebuild count {} must be >=1", rebuilt);
    let optimized = db.optimize_fts();
    assert!(optimized >= 1);

    // Search still works after the maintenance ops.
    let results = db.search_messages("needle", None, None, None, 20, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 1);

    // optimize_fts_storage settles to OK on a fresh v23 DB.
    let out = db.optimize_fts_storage(None, false).unwrap();
    assert_eq!(out["ok"], json!(true), "optimize should settle: {:?}", out);
    db.close();
}

#[test]
fn deferred_rebuild_chunk_loop_indexes_messages() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    for i in 0..30 {
        db.append_message("s1", &msg(if i % 2 == 0 { "user" } else { "assistant" }, &format!("chunkneedle message {}", i)), None).unwrap();
    }

    // Force the deferred-rebuild markers on a populated DB (the state a
    // demote leaves behind), then check the chunk loop drains them.
    let hw: i64 = db
        .writer_conn()
        .query_row("SELECT COALESCE(MAX(id), 0) FROM messages", [], |r| r.get(0))
        .unwrap();
    db.writer_conn()
        .execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_high_water', ?)              ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![hw.to_string()],
        )
        .unwrap();
    db.writer_conn()
        .execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_progress', '0')              ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .unwrap();
    assert!(db.fts_rebuild_status().is_some());

    let mut steps = 0;
    while db.fts_rebuild_step() {
        steps += 1;
        assert!(steps < 100, "chunk loop must terminate");
    }
    // Markers cleared once complete.
    assert!(db.fts_rebuild_status().is_none());
    // The chunk backfill made everything searchable by the FTS path.
    let results = db.search_messages("chunkneedle", None, None, None, 100, 0, None, false, None).unwrap();
    assert_eq!(results.len(), 30);
    db.close();
}

#[test]
fn sort_newest_and_oldest_respected() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default()).unwrap();
    let t0 = 1_700_000_000.0;
    for i in 0..5 {
        db.append_message("s1", &MessageInput {
            role: "user".to_string(),
            content: Some(json!(format!("sortable term msg {}", i))),
            timestamp: Some(t0 + i as f64 * 60.0),
            ..Default::default()
        }, None).unwrap();
    }
    let newest = db.search_messages("sortable", None, None, None, 20, 0, Some("newest"), false, None).unwrap();
    let oldest = db.search_messages("sortable", None, None, None, 20, 0, Some("oldest"), false, None).unwrap();
    let newest_ts: Vec<f64> = newest.iter().map(|r| r["timestamp"].as_f64().unwrap()).collect();
    let oldest_ts: Vec<f64> = oldest.iter().map(|r| r["timestamp"].as_f64().unwrap()).collect();
    assert!(newest_ts.windows(2).all(|w| w[0] >= w[1]));
    assert!(oldest_ts.windows(2).all(|w| w[0] <= w[1]));
    db.close();
}
