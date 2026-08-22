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
