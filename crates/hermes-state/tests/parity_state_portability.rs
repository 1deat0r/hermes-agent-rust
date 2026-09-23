//! Parity oracles for the SessionPortabilityMixin + portability dependencies
//! (rich rows, export/import, lineage, search_sessions), mirroring upstream
//! @ b9aa928:
//!   tests/test_hermes_state.py (TestListCronJobRuns, TestCompactRows,
//!     TestDeleteAndExport.import-session guards, export/import)
//!   tests/test_session_system_prompt_dedup.py (import prompt dedup)
//!   tests/test_session_skill_previews.py

use std::collections::HashSet;
use std::path::PathBuf;

use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::portability::ImportResult;
use hermes_state::state::SessionDB;
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

fn seed_run(db: &SessionDB, job_id: &str, idx: i64, started_at: f64) {
    let sid = format!("cron_{}_{:08}", job_id, idx);
    db.create_session(&sid, "cron", &NewSession::default())
        .unwrap();
    db.append_message(
        &sid,
        &msg("user", &format!("run {} for {}", idx, job_id)),
        None,
    )
    .unwrap();
    db.append_message(&sid, &msg("assistant", "done"), None)
        .unwrap();
    db.end_session(&sid, "completed").unwrap();
    db.writer_conn()
        .execute(
            "UPDATE sessions SET started_at = ? WHERE id = ?",
            rusqlite::params![started_at, sid],
        )
        .unwrap();
}

#[test]
fn list_cron_job_runs_scopes_newest_first_and_enriched() {
    // TestListCronJobRuns::test_scopes_to_job_newest_first_and_enriched
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let base = 1_700_000_000.0f64;
    for i in 0..5 {
        seed_run(&db, "alpha", i, base + i as f64 * 60.0);
    }
    for i in 0..3 {
        seed_run(&db, "beta", i, base + i as f64 * 60.0);
    }

    let runs = db.list_cron_job_runs("alpha", 20, 0).unwrap();
    assert_eq!(runs.len(), 5);
    assert!(runs
        .iter()
        .all(|r| r["id"].as_str().unwrap().starts_with("cron_alpha_")));
    let sts: Vec<f64> = runs
        .iter()
        .map(|r| r["started_at"].as_f64().unwrap())
        .collect();
    let mut sorted = sts.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(sts, sorted);
    assert!(runs[0]["preview"]
        .as_str()
        .unwrap()
        .starts_with("run 4 for alpha"));
    assert!(runs[0]["last_active"].as_f64().unwrap() >= runs[0]["started_at"].as_f64().unwrap());
    db.close();
}

#[test]
fn list_cron_job_runs_pages() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let base = 1_700_000_000.0f64;
    for i in 0..10 {
        seed_run(&db, "alpha", i, base + i as f64 * 60.0);
    }
    let page1 = db.list_cron_job_runs("alpha", 4, 0).unwrap();
    let page2 = db.list_cron_job_runs("alpha", 4, 4).unwrap();
    assert_eq!(page1.len(), 4);
    assert_eq!(page2.len(), 4);
    let ids1: HashSet<&str> = page1.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let ids2: HashSet<&str> = page2.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids1.is_disjoint(&ids2));
    let combined: Vec<f64> = page1
        .iter()
        .chain(&page2)
        .map(|r| r["started_at"].as_f64().unwrap())
        .collect();
    let mut sorted = combined.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(combined, sorted);
    db.close();
}

#[test]
fn rich_row_compact_omits_system_prompt_keeps_git_fields() {
    // TestCompactRows single-row + batch paths
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session(
        "s1",
        "cli",
        &NewSession {
            model: Some("m".to_string()),
            system_prompt: Some("big blob ".repeat(500)),
            ..Default::default()
        },
    )
    .unwrap();
    db.update_session_cwd("s1", "/tmp/w1", Some("main"), Some("/tmp/w1"), true)
        .unwrap();

    // Full row: system_prompt present.
    let full = db.get_session_rich_row("s1", false).unwrap().expect("row");
    assert_eq!(full["id"], json!("s1"));
    assert!(full.get("system_prompt").is_some());
    assert_eq!(full["git_branch"], json!("main"));

    // Compact row: system_prompt gone, git fields kept.
    let row = db.get_session_rich_row("s1", true).unwrap().expect("row");
    assert!(row.get("system_prompt").is_none());
    assert!(row.get("system_prompt_hash").is_none());
    assert_eq!(row["id"], json!("s1"));
    assert_eq!(row["git_branch"], json!("main"));
    assert_eq!(row["git_repo_root"], json!("/tmp/w1"));

    // Batch: missing ids absent from the map.
    let mut batch = db
        .get_session_rich_rows_batch(&["s1".to_string(), "missing".to_string()], true)
        .unwrap();
    assert_eq!(batch.len(), 1);
    let b1 = batch.remove("s1").expect("s1 in batch");
    assert!(b1.get("system_prompt").is_none());
    assert_eq!(b1["preview"], json!(""));
    db.close();
}

#[test]
fn list_skill_scaffolded_and_first_assistant_text() {
    // Skill-scaffold + first-assistant-text helpers (preview stream family)
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let sid = "sk1";
    db.create_session(sid, "cli", &NewSession::default())
        .unwrap();
    db.append_message(
        sid,
        &MessageInput {
            role: "user".to_string(),
            content: Some(json!(
                "[IMPORTANT: The user has invoked the /remember skill] remember that I like tea"
            )),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    db.append_message(sid, &msg("assistant", "plain first reply"), None)
        .unwrap();
    db.set_session_title(sid, "skill title").unwrap();

    let scaffolded = db.list_skill_scaffolded_sessions(200).unwrap();
    assert_eq!(scaffolded.len(), 1);
    assert_eq!(scaffolded[0]["id"], json!("sk1"));
    assert_eq!(scaffolded[0]["title"], json!("skill title"));
    assert!(scaffolded[0]["content"]
        .as_str()
        .unwrap()
        .contains("/remember"));

    assert_eq!(
        db.get_first_assistant_text(sid).unwrap(),
        "plain first reply"
    );
    db.close();
}

#[test]
fn get_first_assistant_text_empty_for_no_assistant() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default())
        .unwrap();
    db.append_message("s1", &msg("user", "only user"), None)
        .unwrap();
    assert_eq!(db.get_first_assistant_text("s1").unwrap(), "");
    assert_eq!(db.get_first_assistant_text("ghost").unwrap(), "");
    db.close();
}

#[test]
fn distinct_session_cwds_aggregates_and_respects_archived() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    for sid in ["a1", "a2", "b1"] {
        db.create_session(sid, "cli", &NewSession::default())
            .unwrap();
    }
    db.update_session_cwd("a1", "/work/proj", None, Some("/work/proj"), true)
        .unwrap();
    db.update_session_cwd("a2", "/work/proj", None, Some("/work/proj"), true)
        .unwrap();
    db.update_session_cwd("b1", "/tmp", None, None, true)
        .unwrap();

    let all = db.distinct_session_cwds(false).unwrap();
    assert_eq!(all.len(), 2);
    let proj = all
        .iter()
        .find(|r| r["cwd"] == json!("/work/proj"))
        .unwrap();
    assert_eq!(proj["sessions"], json!(2));
    assert!(proj["last_active"].as_f64().unwrap() > 0.0);
    // Archived rows are excluded by default.
    db.writer_conn()
        .execute("UPDATE sessions SET archived = 1 WHERE id = 'a1'", [])
        .unwrap();
    let live = db.distinct_session_cwds(false).unwrap();
    let proj_live = live
        .iter()
        .find(|r| r["cwd"] == json!("/work/proj"))
        .unwrap();
    assert_eq!(proj_live["sessions"], json!(1));
    let with_archived = db.distinct_session_cwds(true).unwrap();
    let proj_with = with_archived
        .iter()
        .find(|r| r["cwd"] == json!("/work/proj"))
        .unwrap();
    assert_eq!(proj_with["sessions"], json!(2));
    db.close();
}

#[test]
fn export_session_roundtrips_through_import() {
    // End-to-end export → import of a session with a shared prompt.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session(
        "s1",
        "cli",
        &NewSession {
            model: Some("m".to_string()),
            system_prompt: Some("exported prompt".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    db.append_messages_batch(
        "s1",
        &[msg("user", "q1"), msg("assistant", "a1")],
        None,
        None,
    )
    .unwrap();

    let exported = db.export_session("s1").unwrap().expect("exported");
    assert_eq!(exported["id"], json!("s1"));
    assert_eq!(exported["system_prompt"], json!("exported prompt"));
    assert_eq!(exported["messages"].as_array().unwrap().len(), 2);
    assert_eq!(exported["messages"][0]["content"], json!("q1"));

    // Import into a fresh DB.
    let (_dir2, path2) = tmp_db("target.db");
    let target = SessionDB::open(Some(path2), false).expect("open");
    let result: ImportResult = target.import_sessions(&[exported]).unwrap();
    assert!(result.ok, "import failed: {:?}", result.errors);
    assert_eq!(result.imported, 1);
    let got = target.export_session("s1").unwrap().expect("re-export");
    assert_eq!(got["messages"].as_array().unwrap().len(), 2);
    assert_eq!(got["system_prompt"], json!("exported prompt"));
    assert_eq!(got["model"], json!("m"));
    // Existing id is skipped on re-import.
    let again = target.import_sessions(&[got]).unwrap();
    assert!(again.ok);
    assert_eq!(again.skipped, 1);
    target.close();
    db.close();
}

#[test]
fn import_deduplicates_shared_prompts() {
    // test_imported_prompts_are_deduplicated
    let (_dir, path) = tmp_db("source.db");
    let source = SessionDB::open(Some(path), false).expect("open");
    let prompt = "shared imported prompt";
    source
        .create_session(
            "s1",
            "cli",
            &NewSession {
                system_prompt: Some(prompt.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    source
        .create_session(
            "s2",
            "telegram",
            &NewSession {
                system_prompt: Some(prompt.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    let exported = [
        source.export_session("s1").unwrap().unwrap(),
        source.export_session("s2").unwrap().unwrap(),
    ];
    source.close();

    let (_dir2, path2) = tmp_db("target.db");
    let target = SessionDB::open(Some(path2), false).expect("open");
    let result = target.import_sessions(&exported).unwrap();
    assert!(result.ok);
    assert_eq!(result.imported, 2);
    let hashes: HashSet<String> = target
        .writer_conn()
        .prepare("SELECT system_prompt_hash FROM sessions")
        .unwrap()
        .query_map([], |r| r.get::<_, Option<String>>(0))
        .unwrap()
        .filter_map(Result::ok)
        .flatten()
        .collect();
    assert_eq!(hashes.len(), 1);
    assert_eq!(
        target
            .get_session("s1")
            .unwrap()
            .unwrap()
            .system_prompt
            .as_deref(),
        Some(prompt)
    );
    assert_eq!(
        target
            .get_session("s2")
            .unwrap()
            .unwrap()
            .system_prompt
            .as_deref(),
        Some(prompt)
    );
    target.close();
}

#[test]
fn import_rejects_oversized_payloads_atomically() {
    // TestDeleteAndExport::test_import_sessions_rejects_oversized_payloads_atomically
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");

    let oversized = "x".repeat(5 * 1024 * 1024 + 1);
    let r = db
        .import_sessions(&[
            json!({"id": "oversized", "messages": [{"role": "user", "content": oversized}]}),
        ])
        .unwrap();
    assert!(!r.ok);
    assert_eq!(
        r.errors[0]["error"],
        "session exceeds the import size limit"
    );
    assert!(db.get_session("oversized").unwrap().is_none());

    let many: Vec<Value> = (0..10_001)
        .map(|i| json!({"role": "user", "content": format!("x{}", i)}))
        .collect();
    let r = db
        .import_sessions(&[json!({"id": "too-many-messages", "messages": many})])
        .unwrap();
    assert!(!r.ok);
    assert_eq!(
        r.errors[0]["error"],
        "messages exceeds the per-session import limit"
    );
    assert!(db.get_session("too-many-messages").unwrap().is_none());
    db.close();
}

#[test]
fn import_wires_parents_and_detaches_missing() {
    // Partial-import contract: children keep the parent only if it exists or
    // is in the same payload; otherwise the closing edge is dropped.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");

    // Parent + child in the same payload: wired.
    let r = db
        .import_sessions(&[
            json!({"id": "p", "source": "cli", "messages": [{"role": "user", "content": "p1"}]}),
            json!({"id": "c", "source": "cli", "parent_session_id": "p", "messages": [{"role": "user", "content": "c1"}]}),
        ])
        .unwrap();
    assert!(r.ok, "import failed: {:?}", r.errors);
    assert_eq!(r.imported, 2);
    assert_eq!(r.detached, 0);
    assert_eq!(
        db.get_session("c")
            .unwrap()
            .unwrap()
            .parent_session_id
            .as_deref(),
        Some("p")
    );

    // Child with a missing parent in another payload: detached edge.
    let r = db
        .import_sessions(&[json!({"id": "orphan-child", "source": "cli", "parent_session_id": "no-parent", "messages": [{"role": "user", "content": "x"}]})])
        .unwrap();
    assert!(r.ok);
    assert_eq!(r.detached, 1);
    assert!(db
        .get_session("orphan-child")
        .unwrap()
        .unwrap()
        .parent_session_id
        .is_none());

    // Cycle in the payload: the closing edge is dropped, not committed.
    let r = db
        .import_sessions(&[
            json!({"id": "a", "source": "cli", "parent_session_id": "b", "messages": [{"role": "user", "content": "a1"}]}),
            json!({"id": "b", "source": "cli", "parent_session_id": "a", "messages": [{"role": "user", "content": "b1"}]}),
        ])
        .unwrap();
    assert!(r.ok);
    // One of the two edges must have been detached (the cycle cannot commit).
    assert_eq!(r.detached, 1);
    let pa = db.get_session("a").unwrap().unwrap().parent_session_id;
    let pb = db.get_session("b").unwrap().unwrap().parent_session_id;
    let cycle_edges = (pa.is_some() && pb == Some("a".to_string()))
        || (pb.is_some() && pa == Some("b".to_string()));
    assert!(!cycle_edges, "cycle must never commit both edges");
    db.close();
}

#[test]
fn search_sessions_mru_order_workspace_key_and_enrichment() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("old", "cli", &NewSession::default())
        .unwrap();
    db.create_session("new", "cli", &NewSession::default())
        .unwrap();
    db.append_message("old", &msg("user", "old turn"), None)
        .unwrap();
    db.append_message("new", &msg("user", "fresh turn"), None)
        .unwrap();
    db.update_session_cwd("new", "/repo/src", Some("main"), Some("/repo"), true)
        .unwrap();

    let all = db.search_sessions(None, 100, 0, None).unwrap();
    let ids: Vec<&str> = all.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["new", "old"]);
    assert!(all[0].get("last_active").is_some());
    assert_eq!(all[0]["system_prompt"], Value::Null); // resolved fold keeps key

    // Workspace scoping: git_repo_root="/repo"; cwd under it also matches.
    let ws = db.search_sessions(None, 100, 0, Some("/repo")).unwrap();
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0]["id"], json!("new"));
    // Source filter.
    db.create_session("tg", "telegram", &NewSession::default())
        .unwrap();
    let src = db.search_sessions(Some("telegram"), 100, 0, None).unwrap();
    assert_eq!(
        src.iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["tg"]
    );
    db.close();
}

#[test]
fn get_compression_lineage_through_tip_and_fork_shortcut() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    // root ->(compression)-> mid ->(compression)-> tip
    db.create_session("root", "cli", &NewSession::default())
        .unwrap();
    db.append_message("root", &msg("user", "root turn"), None)
        .unwrap();
    db.end_session("root", "compression").unwrap();
    db.create_session(
        "mid",
        "cli",
        &NewSession {
            parent_session_id: Some("root".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    db.append_message("mid", &msg("user", "mid turn"), None)
        .unwrap();
    db.end_session("mid", "compression").unwrap();
    db.create_session(
        "tip",
        "cli",
        &NewSession {
            parent_session_id: Some("mid".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    db.append_message("tip", &msg("user", "tip turn"), None)
        .unwrap();

    assert_eq!(
        db.get_compression_lineage("tip").unwrap(),
        vec!["root", "mid", "tip"]
    );
    assert_eq!(
        db.get_compression_lineage("mid").unwrap(),
        vec!["root", "mid", "tip"]
    );
    // An explicit fork child (tool source) is its own lineage.
    db.create_session(
        "fork",
        "tool",
        &NewSession {
            parent_session_id: Some("tip".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(db.get_compression_lineage("fork").unwrap(), vec!["fork"]);

    // export_session_lineage merges segments.
    let merged = db.export_session_lineage("tip").unwrap().unwrap();
    assert_eq!(merged["lineage_session_ids"].as_array().unwrap().len(), 3);
    assert_eq!(merged["message_count"], json!(3));
    assert_eq!(merged["messages"].as_array().unwrap().len(), 3);
    assert_eq!(merged["id"], json!("tip"));
    db.close();
}

#[test]
fn export_all_lists_message_bearing_sessions() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("one", "cli", &NewSession::default())
        .unwrap();
    db.append_message("one", &msg("user", "hello"), None)
        .unwrap();
    db.create_session("two", "telegram", &NewSession::default())
        .unwrap();
    db.append_messages_batch(
        "two",
        &[msg("user", "hi"), msg("assistant", "yo")],
        None,
        None,
    )
    .unwrap();

    let all = db.export_all(None).unwrap();
    assert_eq!(all.len(), 2);
    let one = all.iter().find(|s| s["id"] == json!("one")).unwrap();
    assert_eq!(one["messages"].as_array().unwrap().len(), 1);
    let two = all.iter().find(|s| s["id"] == json!("two")).unwrap();
    assert_eq!(two["messages"].as_array().unwrap().len(), 2);
    // export_all(source=...) filters.
    let cli = db.export_all(Some("cli")).unwrap();
    assert_eq!(cli.len(), 1);
    assert_eq!(cli[0]["id"], json!("one"));
    db.close();
}

// =====================================================================
// Oracle: tests/hermes_state/test_hermes_state_ids.py (SITES entry for
// hermes_state_portability) + the shared mint helper.
// =====================================================================

#[test]
fn oracle_new_session_id_shapes_match_salvage_pattern() {
    // test_minted_ids_are_what_salvage_classifies_as_session_ids — the
    // fixed `YYYYMMDD_HHMMSS_` prefix plus the caller-chosen hex width.
    use hermes_state::ids::{new_session_id, SESSION_ID_PATTERN};
    for hex_len in [6usize, 8, 12] {
        let id = new_session_id(hex_len);
        let re = regex::Regex::new(&format!(r"^\d{{8}}_\d{{6}}_[0-9a-f]{{{hex_len}}}$")).unwrap();
        assert!(re.is_match(&id), "hex_len {hex_len}: {id}");
        assert!(SESSION_ID_PATTERN.is_match(&id), "{id}");
    }
    // The portability module re-exports THE one helper (oracle SITES entry:
    // `("hermes_state_portability", None)` — minting there must resolve to
    // the ids-module function).
    let from_portability = hermes_state::portability::new_session_id(12);
    let re = regex::Regex::new(r"^\d{8}_\d{6}_[0-9a-f]{12}$").unwrap();
    assert!(re.is_match(&from_portability), "{from_portability}");
    let re8 = regex::Regex::new(r"^\d{8}_\d{6}_[0-9a-f]{8}$").unwrap();
    assert!(re8.is_match(&hermes_state::ids::new_session_id(8)));
}

// =====================================================================
// Oracle: tests/hermes_state/test_session_export_timings.py
// =====================================================================

fn ts_msg(role: &str, content: &str, timestamp: f64) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(json!(content)),
        timestamp: Some(timestamp),
        ..Default::default()
    }
}

#[test]
fn oracle_export_session_includes_text_free_timing_evidence() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default())
        .unwrap();
    db.append_message("s1", &ts_msg("user", "secret prompt", 1000.0), None)
        .unwrap();
    let mut tool_turn = ts_msg("assistant", "", 1001.25);
    tool_turn.tool_calls = Some(json!([{"id": "call-1", "function": {"name": "terminal"}}]));
    db.append_message("s1", &tool_turn, None).unwrap();
    let mut tool_result = ts_msg("tool", "secret tool output", 1003.0);
    tool_result.tool_name = Some("terminal".to_string());
    tool_result.tool_call_id = Some("call-1".to_string());
    db.append_message("s1", &tool_result, None).unwrap();
    db.append_message("s1", &ts_msg("assistant", "done", 1003.5), None)
        .unwrap();

    let exported = db.export_session("s1").unwrap().expect("exported");
    db.close();

    let timings = &exported["timings"];
    assert_eq!(timings["source"], json!("message_timestamps"));
    assert_eq!(timings["available"], json!(true));
    assert_eq!(timings["complete"], json!(false));
    assert_eq!(timings["unavailable_reason"], Value::Null);
    assert_eq!(timings["wall_clock_ms"], json!(3500));
    assert_eq!(timings["largest_gap_ms"], json!(1750));
    assert_eq!(
        timings["message_timestamps"],
        json!({"available": 4, "missing": 0})
    );
    assert_eq!(
        timings["role_counts"],
        json!({"user": 1, "assistant": 2, "tool": 1})
    );
    assert_eq!(timings["tool_calls_emitted"], json!(1));
    assert_eq!(timings["tool_result_count"], json!(1));
    assert_eq!(
        timings["intervals"],
        json!([
            {"from_message_id": 1, "to_message_id": 2,
             "from_role": "user", "to_role": "assistant", "gap_ms": 1250},
            {"from_message_id": 2, "to_message_id": 3,
             "from_role": "assistant", "to_role": "tool", "gap_ms": 1750},
            {"from_message_id": 3, "to_message_id": 4,
             "from_role": "tool", "to_role": "assistant", "gap_ms": 500},
        ])
    );
    // Text-free contract: ids/roles/counts/durations only.
    let blob = timings.to_string();
    assert!(!blob.contains("secret prompt"), "{blob}");
    assert!(!blob.contains("secret tool output"), "{blob}");
}

#[test]
fn oracle_lineage_timings_span_merged_and_import_ignores_their_size() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("root", "cli", &NewSession::default())
        .unwrap();
    db.append_message("root", &ts_msg("user", "first", 100.0), None)
        .unwrap();
    db.append_message("root", &ts_msg("assistant", "ok", 101.0), None)
        .unwrap();
    db.end_session("root", "compression").unwrap();
    db.create_session(
        "child",
        "cli",
        &NewSession {
            parent_session_id: Some("root".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    db.append_message("child", &ts_msg("user", "second", 200.0), None)
        .unwrap();
    db.append_message("child", &ts_msg("assistant", "done", 200.5), None)
        .unwrap();

    let mut exported = db
        .export_session_lineage("child")
        .unwrap()
        .expect("lineage");
    assert_eq!(exported["lineage_session_ids"], json!(["root", "child"]));
    assert_eq!(exported["timings"]["wall_clock_ms"], json!(100_500));
    assert_eq!(
        exported["timings"]["message_timestamps"],
        json!({"available": 4, "missing": 0})
    );
    assert_eq!(
        exported["segments"][1]["timings"]["wall_clock_ms"],
        json!(500)
    );

    // A padded timings block (~6 MiB) must not eat the 5 MiB session budget
    // — import strips the derived key before measuring.
    exported["timings"]["intervals"] = Value::Array(vec![json!({"pad": "x".repeat(100)}); 60_000]);
    let (_dir2, path2) = tmp_db("target.db");
    let target = SessionDB::open(Some(path2), false).expect("open");
    let report = target
        .import_sessions(std::slice::from_ref(&exported))
        .unwrap();
    assert!(report.errors.is_empty());
    assert_eq!(report.imported, 1);
    target.close();
    db.close();
}

#[test]
fn oracle_corrupt_timestamp_rows_count_as_missing_instead_of_aborting_export() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("s1", "cli", &NewSession::default())
        .unwrap();
    db.append_message("s1", &ts_msg("user", "hello", 10.0), None)
        .unwrap();
    db.append_message("s1", &ts_msg("assistant", "hi", 11.0), None)
        .unwrap();
    // Writers refuse bad stamps; emulate a pre-existing corrupt row directly
    // (8.4e252 is finite f64 but far past the EPOCH_MAX bound).
    db.writer_conn()
        .execute("UPDATE messages SET timestamp = 8.4e252 WHERE id = 2", [])
        .unwrap();
    let exported = db.export_session("s1").unwrap().expect("export");
    db.close();

    let timings = &exported["timings"];
    assert_eq!(
        timings["message_timestamps"],
        json!({"available": 1, "missing": 1})
    );
    assert_eq!(timings["wall_clock_ms"], json!(0));
    assert_eq!(timings["intervals"], json!([]));
}

// =====================================================================
// Oracle (code-is-oracle): foreign import — hermes_state_portability
// lines 128–173; consumer shape from tests/hermes_cli
// test_foreign_sessions.py::test_import_*.
// =====================================================================

#[test]
fn foreign_import_mints_12hex_id_stamps_origin_and_is_idempotent() {
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let origin = json!({
        "tool": "claude-code",
        "path": "/home/user/.claude/sessions/x.jsonl",
        "foreign_session_id": "sess-abc",
    });
    let turns = [
        json!({"role": "user", "content": "q1", "timestamp": 10.0}),
        json!({"role": "assistant", "content": "a1", "timestamp": 11.0}),
    ];

    let result = db
        .import_foreign_history(
            &origin,
            &turns,
            "Imported from Claude Code: please fix",
            Some("/home/user/proj"),
            "developer",
        )
        .expect("import");
    assert_eq!(result["already_imported"], json!(false));
    let session_id = result["session_id"].as_str().unwrap().to_string();
    // Portability imports mint 12-hex ids (oracle: `new_session_id(hex_len=12)`).
    let re = regex::Regex::new(r"^\d{8}_\d{6}_[0-9a-f]{12}$").unwrap();
    assert!(re.is_match(&session_id), "{session_id}");

    let row = db.get_session_dict(&session_id).unwrap().expect("row");
    assert_eq!(row["source"], json!("claude-code"));
    assert_eq!(row["cwd"], json!("/home/user/proj"));
    assert_eq!(row["message_count"], json!(2));
    assert_eq!(row["profile_name"], json!("developer"));
    let imported_from = &row["origin_json"]
        .as_str()
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap()["imported_from"];
    assert_eq!(imported_from["tool"], json!("claude-code"));
    assert_eq!(
        imported_from["path"],
        json!("/home/user/.claude/sessions/x.jsonl")
    );
    assert_eq!(imported_from["foreign_session_id"], json!("sess-abc"));
    assert_eq!(
        db.get_messages_dicts(&session_id, false, None, 0)
            .unwrap()
            .len(),
        2
    );

    // find_foreign_import locates the adopted row by foreign id…
    assert_eq!(
        db.find_foreign_import(&origin).unwrap().as_deref(),
        Some(session_id.as_str())
    );
    // …and by path when no foreign id is supplied.
    let by_path = json!({"tool": "claude-code", "path": "/home/user/.claude/sessions/x.jsonl"});
    assert_eq!(
        db.find_foreign_import(&by_path).unwrap().as_deref(),
        Some(session_id.as_str())
    );
    // A different tool never matches an existing import.
    let other = json!({"tool": "codex-cli", "path": "/elsewhere.jsonl"});
    assert!(db.find_foreign_import(&other).unwrap().is_none());

    // Second click: already-imported, same id, no duplicate row.
    let again = db
        .import_foreign_history(
            &origin,
            &turns,
            "Imported from Claude Code: please fix",
            Some("/home/user/proj"),
            "developer",
        )
        .unwrap();
    assert_eq!(again["already_imported"], json!(true));
    assert_eq!(again["session_id"], json!(session_id));
    db.close();
}

#[test]
fn foreign_import_disambiguates_duplicate_titles() {
    // Titles are globally unique within a profile; a collision mints
    // `"{title} ({session_id[-12:]})"`.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("existing", "cli", &NewSession::default())
        .unwrap();
    db.set_session_title("existing", "Bot Chat").unwrap();

    let origin = json!({"tool": "codex-cli", "path": "/x.jsonl"});
    let turns = [json!({"role": "user", "content": "hi"})];
    let first = db
        .import_foreign_history(&origin, &turns, "Bot Chat", None, "")
        .unwrap();
    let new_id = first["session_id"].as_str().unwrap().to_string();
    let title = db.get_session_title(&new_id).unwrap().expect("title");
    assert!(
        title.starts_with("Bot Chat (") && title.ends_with(')'),
        "collided title carries the id suffix: {title}"
    );
    assert_eq!(title.len(), "Bot Chat ()".len() + 12);
    // The original keeps its title.
    assert_eq!(
        db.get_session_title("existing").unwrap().as_deref(),
        Some("Bot Chat")
    );
    db.close();
}

#[test]
fn foreign_import_rejects_bad_roles() {
    // `_normalize_import_session`: role must be a non-empty string —
    // upstream raises ValueError before any write.
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    let origin = json!({"tool": "claude-code", "path": "/x.jsonl"});
    let turns = [json!({"role": "", "content": "hi"})];
    let err = db
        .import_foreign_history(&origin, &turns, "T", None, "")
        .expect_err("must reject");
    assert!(
        err.to_string()
            .contains("messages[0].role must be a non-empty string"),
        "{err}"
    );
    db.close();
}

// =====================================================================
// Oracle: tests/tui_gateway/test_stranded_session_adoption.py
// (unit-level `stores` fixture half — handler/gateway rows skipped).
// The TOCTOU export-patch case is skipped: Python monkeypatches the
// donor's instance method mid-call; the Rust `&SessionDB` seam has no
// equivalent without a trait split (see PORT SEAMS in portability.rs).
// =====================================================================

fn seed_stranded(db: &SessionDB, session_id: &str, turns: usize, title: &str) {
    db.create_session(session_id, "tui", &NewSession::default())
        .unwrap();
    db.set_session_title(session_id, title).unwrap();
    for i in 1..=turns {
        db.append_message(session_id, &msg("user", &format!("question {i}")), None)
            .unwrap();
        db.append_message(session_id, &msg("assistant", &format!("answer {i}")), None)
            .unwrap();
    }
}

struct Stores {
    _dir: tempfile::TempDir,
    default_path: PathBuf,
    default_db: SessionDB,
    profile_db: SessionDB,
}

fn stores() -> Stores {
    let dir = tempfile::tempdir().expect("tempdir");
    let default_path = dir.path().join("state.db");
    let profile_home = dir.path().join("profiles/developer");
    std::fs::create_dir_all(&profile_home).unwrap();
    let default_db = SessionDB::open(Some(default_path.clone()), false).expect("open");
    let profile_db = SessionDB::open(Some(profile_home.join("state.db")), false).expect("open");
    Stores {
        _dir: dir,
        default_path,
        default_db,
        profile_db,
    }
}

const STRANDED: &str = "20260823_043331_c93770";

#[test]
fn adoption_moves_session_and_messages() {
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");

    let result = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .expect("adopt");
    assert_eq!(result["adopted"], json!(true));
    assert_eq!(result["imported"], json!(1));
    assert!(s.profile_db.get_session(STRANDED).unwrap().is_some());
    let msgs = s
        .profile_db
        .get_messages_dicts(STRANDED, false, None, 0)
        .unwrap();
    assert_eq!(msgs.len(), 6);
    assert_eq!(msgs[0]["content"], json!("question 1"));
    assert_eq!(msgs[5]["content"], json!("answer 3"));
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn adoption_retires_donor_archived_not_deleted() {
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");

    s.profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();

    let donor = s.default_db.get_session(STRANDED).unwrap().expect("donor");
    assert!(donor.archived, "archived, never deleted");
    assert_eq!(donor.end_reason.as_deref(), Some("adopted_by_profile"));
    // Bytes stay recoverable.
    assert_eq!(
        s.default_db
            .get_messages_dicts(STRANDED, false, None, 0)
            .unwrap()
            .len(),
        6
    );
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn adoption_archive_is_not_recoverable_resurrectable() {
    // test_adoption_archive_is_not_recoverable_resurrectable — the
    // adoption stamp must fall outside the recoverable set, so
    // unarchive_recoverable_session refuses it.
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");
    s.profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();

    assert!(
        !hermes_state::common::RECOVERABLE_END_REASONS.contains(&"adopted_by_profile"),
        "adoption is deliberately non-recoverable"
    );
    assert!(!s
        .default_db
        .unarchive_recoverable_session(STRANDED)
        .unwrap());
    let donor = s.default_db.get_session(STRANDED).unwrap().expect("donor");
    assert!(donor.archived, "still archived after the refused unarchive");
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn adoption_is_idempotent() {
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");

    let first = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();
    let second = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();
    assert_eq!(first["adopted"], json!(true));
    assert_eq!(second["adopted"], json!(true));
    assert_eq!(second["imported"], json!(0));
    assert_eq!(second["skipped"], json!(1));
    assert_eq!(
        s.profile_db
            .get_messages_dicts(STRANDED, false, None, 0)
            .unwrap()
            .len(),
        6
    );
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn missing_donor_session_is_reported_not_raised() {
    let s = stores();
    let result = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, "nope", true)
        .unwrap();
    assert_eq!(result["adopted"], json!(false));
    assert!(
        result["error"].as_str().unwrap().contains("not found"),
        "{}",
        result
    );
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn compression_lineage_adopts_as_a_unit() {
    let s = stores();
    let parent = "sess-parent";
    let child = "sess-child";
    seed_stranded(&s.default_db, parent, 2, "Bot Chat");
    s.default_db.end_session(parent, "compression").unwrap();
    s.default_db
        .create_session(
            child,
            "tui",
            &NewSession {
                parent_session_id: Some(parent.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    s.default_db.set_session_title(child, "Bot Chat").unwrap();
    s.default_db
        .append_message(child, &msg("user", "post-compaction question"), None)
        .unwrap();
    s.default_db
        .append_message(child, &msg("assistant", "post-compaction answer"), None)
        .unwrap();

    let result = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, parent, true)
        .unwrap();
    assert_eq!(result["adopted"], json!(true));
    assert_eq!(result["imported"], json!(2));
    assert!(s.profile_db.get_session(parent).unwrap().is_some());
    assert!(s.profile_db.get_session(child).unwrap().is_some());
    let child_row = s.profile_db.get_session(child).unwrap().expect("child");
    assert_eq!(child_row.parent_session_id.as_deref(), Some(parent));
    for sid in [parent, child] {
        let donor = s.default_db.get_session(sid).unwrap().expect("donor");
        assert!(donor.archived, "{sid} must be retired in donor store");
    }
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn adoption_does_not_touch_unrelated_sessions() {
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");
    seed_stranded(&s.default_db, "other-session", 3, "Other Chat");

    s.profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();

    let other = s
        .default_db
        .get_session("other-session")
        .unwrap()
        .expect("other");
    assert!(!other.archived, "unrelated session untouched");
    assert!(s.profile_db.get_session("other-session").unwrap().is_none());
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn divergent_donor_is_not_retired() {
    // test_divergent_donor_is_not_retired — donor grew after a completed
    // adoption; re-adoption must refuse retirement (export-time guard).
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");
    let first = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();
    assert_eq!(first["adopted"], json!(true));
    assert_eq!(first["donor_retired"], json!(true));

    // Donor keeps living — un-retire + append.
    s.default_db.set_session_archived(STRANDED, false).unwrap();
    s.default_db
        .append_message(STRANDED, &msg("user", "late question"), None)
        .unwrap();
    s.default_db
        .append_message(STRANDED, &msg("assistant", "late answer"), None)
        .unwrap();

    let second = s
        .profile_db
        .adopt_session_lineage_from(&s.default_db, STRANDED, true)
        .unwrap();
    assert_eq!(second["adopted"], json!(true));
    assert_eq!(second["donor_retired"], json!(false));
    let donor = s.default_db.get_session(STRANDED).unwrap().expect("donor");
    assert!(!donor.archived, "diverged donor must stay reachable");
    assert_eq!(
        s.default_db
            .get_messages_dicts(STRANDED, false, None, 0)
            .unwrap()
            .len(),
        8
    );
    s.default_db.close();
    s.profile_db.close();
}

#[test]
fn retire_failure_reports_false_readonly_donor() {
    // M1: donor_retired must not lie when retirement fails. The Python
    // oracle monkeypatches end_session; the Rust seam forces the same
    // failure by retiring through a READ-ONLY donor connection
    // (execute_write cannot BEGIN on a read-only handle).
    let s = stores();
    seed_stranded(&s.default_db, STRANDED, 3, "Bot Chat");
    let path = s.default_path.clone();
    s.default_db.close();

    let ro = SessionDB::open(Some(path.clone()), true).expect("read-only donor");
    let result = s
        .profile_db
        .adopt_session_lineage_from(&ro, STRANDED, true)
        .unwrap();
    assert_eq!(result["adopted"], json!(true));
    assert_eq!(result["donor_retired"], json!(false));
    // Reopen writable to inspect: not archived (retirement failed honestly).
    drop(ro);
    let donor = SessionDB::open(Some(path), false).expect("reopen");
    let row = donor.get_session(STRANDED).unwrap().expect("donor");
    assert!(!row.archived, "retirement failed → not stamped");
    donor.close();
    s.profile_db.close();
}

#[test]
fn unarchive_recoverable_true_arm_clears_end_reason() {
    // The adoption test pins the False arm; the True arm is code-as-oracle
    // (upstream hermes_state_sessions.py unarchive_recoverable_session).
    let (_dir, path) = tmp_db("state.db");
    let db = SessionDB::open(Some(path), false).expect("open");
    db.create_session("accident", "cli", &NewSession::default())
        .unwrap();
    db.end_session("accident", "agent_close").unwrap();
    db.set_session_archived("accident", true).unwrap();

    assert!(db.unarchive_recoverable_session("accident").unwrap());
    let row = db.get_session("accident").unwrap().expect("row");
    assert!(!row.archived);
    assert_eq!(row.end_reason, None, "accidental end stamp cleared");

    // Deliberate archive (no end_reason) is left alone.
    db.create_session("deliberate", "cli", &NewSession::default())
        .unwrap();
    db.set_session_archived("deliberate", true).unwrap();
    assert!(!db.unarchive_recoverable_session("deliberate").unwrap());
    assert!(!db.unarchive_recoverable_session("ghost").unwrap());
    assert!(!db.unarchive_recoverable_session("").unwrap());
    db.close();
}

#[test]
fn recoverable_end_reasons_set_matches_oracle() {
    // hermes_state_common.py `_RECOVERABLE_END_REASONS` (line 164).
    assert_eq!(
        hermes_state::common::RECOVERABLE_END_REASONS,
        [
            "agent_close",
            "ws_orphan_reap",
            "superseded_by_resume",
            "startup_orphan_reap"
        ]
    );
}
