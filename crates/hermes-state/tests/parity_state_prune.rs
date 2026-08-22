//! Parity oracles for the space-reclamation surface, mirroring upstream
//! tests/test_hermes_state.py (TestPruneSessions, TestPruneSessionFilters)
//! and tests/hermes_state/test_session_archiving.py (compression-lineage
//! archive/unarchive) @ b9aa928. The `archived` projections used by the
//! archiving tests' list_sessions_rich assertions land with the surface
//! read-helpers unit; here we assert the `set_session_archived` lineage
//! behavior directly via get_session/raw reads.

use std::path::PathBuf;

use hermes_state::crud::NewSession;
use hermes_state::prune::PruneFilters;
use hermes_state::state::{now, SessionDB};

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

fn update_cols(db: &SessionDB, sid: &str, cols: &[(&str, Option<String>)]) {
    // NULL title columns must stay NULL (sessions.title is UNIQUE); npm
    // bind None as SQL NULL so multiple untitled rows coexist.
    let sets: Vec<String> = cols.iter().map(|(k, _)| format!("{k} = ?")).collect();
    let conn = db.writer_conn();
    let mut s = conn
        .prepare(&format!(
            "UPDATE sessions SET {} WHERE id = ?",
            sets.join(", ")
        ))
        .expect("stmt");
    let n = cols.len() + 1;
    let col_vals: Vec<Option<String>> = cols.iter().map(|(_, v)| v.clone()).collect();
    let sid: Option<String> = Some(sid.to_string());
    let mut vals: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(n);
    for v in &col_vals {
        vals.push(v as &dyn rusqlite::ToSql);
    }
    vals.push(&sid);
    s.execute(rusqlite::params_from_iter(vals)).expect("update");
}

fn mk(
    db: &SessionDB,
    sid: &str,
    source: &str,
    age_seconds: f64,
    title: Option<&str>,
    message_count: i64,
    end_reason: &str,
) {
    db.create_session(sid, source, &NewSession::default())
        .expect("create");
    db.end_session(sid, end_reason).expect("end");
    update_cols(
        db,
        sid,
        &[
            ("started_at", Some(format!("{}", now() - age_seconds))),
            ("message_count", Some(format!("{message_count}"))),
            ("title", title.map(|t| t.to_string())),
        ],
    );
}

fn archived_flag(db: &SessionDB, sid: &str) -> Option<i64> {
    let conn = db.writer_conn();
    conn.query_row(
        "SELECT archived FROM sessions WHERE id = ?",
        rusqlite::params![sid],
        |r| r.get(0),
    )
    .ok()
}

// =====================================================================
// TestPruneSessions
// =====================================================================

#[test]
fn prune_old_ended_sessions() {
    let (_dir, db) = open_db("state.db");
    db.create_session("old", "cli", &NewSession::default())
        .expect("create");
    db.end_session("old", "done").expect("end");
    update_cols(
        &db,
        "old",
        &[("started_at", Some(format!("{}", now() - 100.0 * 86400.0)))],
    );

    db.create_session("new", "cli", &NewSession::default())
        .expect("create");

    let pruned = db
        .prune_sessions(Some(90.0), None, None, PruneFilters::default())
        .expect("prune");
    assert_eq!(pruned, 1);
    assert!(db.get_session("old").expect("get").is_none());
    assert!(db.get_session("new").expect("get").is_some());
    db.close();
}

#[test]
fn prune_skips_active_sessions() {
    let (_dir, db) = open_db("state.db");
    db.create_session("active", "cli", &NewSession::default())
        .expect("create");
    // Backdate but don't end.
    update_cols(
        &db,
        "active",
        &[("started_at", Some(format!("{}", now() - 200.0 * 86400.0)))],
    );

    let pruned = db
        .prune_sessions(Some(90.0), None, None, PruneFilters::default())
        .expect("prune");
    assert_eq!(pruned, 0);
    assert!(db.get_session("active").expect("get").is_some());
    db.close();
}

// =====================================================================
// TestPruneSessionFilters
// =====================================================================

#[test]
fn started_after_window_prunes_only_recent() {
    let (_dir, db) = open_db("state.db");
    mk(&db, "recent1", "cli", 3600.0, None, 0, "done");
    mk(&db, "recent2", "cli", 2.0 * 3600.0, None, 0, "done");
    mk(&db, "old", "cli", 10.0 * 3600.0, None, 0, "done");

    let cutoff = now() - 5.0 * 3600.0;
    let pruned = db
        .prune_sessions(
            None,
            None,
            None,
            PruneFilters {
                started_after: Some(cutoff),
                ..Default::default()
            },
        )
        .expect("prune");
    assert_eq!(pruned, 2);
    assert!(db.get_session("old").expect("get").is_some());
    assert!(db.get_session("recent1").expect("get").is_none());
    db.close();
}

#[test]
fn title_and_message_count_filters() {
    let (_dir, db) = open_db("state.db");
    mk(
        &db,
        "smoke1",
        "cli",
        60.0,
        Some("Codex Smoke Test 1"),
        2,
        "done",
    );
    mk(
        &db,
        "smoke2",
        "cli",
        60.0,
        Some("codex smoke test 2"),
        8,
        "done",
    );
    mk(&db, "real", "cli", 60.0, Some("Debugging auth"), 8, "done");

    let rows = db
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                title_like: Some("smoke".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["smoke1", "smoke2"]);

    let pruned = db
        .prune_sessions(
            None,
            None,
            None,
            PruneFilters {
                title_like: Some("Smoke".into()),
                max_messages: Some(3),
                ..Default::default()
            },
        )
        .expect("prune");
    assert_eq!(pruned, 1);
    assert!(db.get_session("smoke1").expect("get").is_none());
    assert!(db.get_session("smoke2").expect("get").is_some());
    assert!(db.get_session("real").expect("get").is_some());
    db.close();
}

#[test]
fn title_like_underscore_is_literal() {
    let (_dir, db) = open_db("state.db");
    mk(
        &db,
        "target",
        "cli",
        60.0,
        Some("user_auth refactor"),
        0,
        "done",
    );
    mk(
        &db,
        "bystander1",
        "cli",
        60.0,
        Some("user-auth review"),
        0,
        "done",
    );
    mk(
        &db,
        "bystander2",
        "cli",
        60.0,
        Some("userXauth notes"),
        0,
        "done",
    );
    mk(
        &db,
        "bystander3",
        "cli",
        60.0,
        Some("user auth meeting"),
        0,
        "done",
    );

    let rows = db
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                title_like: Some("user_auth".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["target"]);

    let pruned = db
        .prune_sessions(
            None,
            None,
            None,
            PruneFilters {
                title_like: Some("user_auth".into()),
                ..Default::default()
            },
        )
        .expect("prune");
    assert_eq!(pruned, 1);
    for survivor in ["bystander1", "bystander2", "bystander3"] {
        assert!(db.get_session(survivor).expect("get").is_some());
    }
    db.close();
}

#[test]
fn percent_in_filter_does_not_select_everything() {
    let (_dir, db) = open_db("state.db");
    mk(&db, "a", "cli", 60.0, Some("alpha"), 0, "done");
    mk(&db, "b", "cli", 60.0, Some("beta"), 0, "done");
    mk(
        &db,
        "pct",
        "cli",
        60.0,
        Some("100% coverage run"),
        0,
        "done",
    );

    for filter in ["%", "100%"] {
        let rows = db
            .list_prune_candidates(
                None,
                None,
                PruneFilters {
                    title_like: Some(filter.into()),
                    ..Default::default()
                },
            )
            .expect("candidates");
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["pct"], "filter={filter}");
    }
    db.close();
}

#[test]
fn branch_like_underscore_is_literal() {
    let (_dir, db) = open_db("state.db");
    db.create_session("want", "cli", &NewSession::default())
        .expect("create");
    db.end_session("want", "done").expect("end");
    update_cols(
        &db,
        "want",
        &[("git_branch", Some("fix/session_prune".into()))],
    );
    db.create_session("other", "cli", &NewSession::default())
        .expect("create");
    db.end_session("other", "done").expect("end");
    update_cols(
        &db,
        "other",
        &[("git_branch", Some("fix/session-prune".into()))],
    );

    let rows = db
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                branch_like: Some("session_prune".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["want"]);
    db.close();
}

#[test]
fn model_like_underscore_is_literal() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "want",
        "cli",
        &NewSession {
            model: Some("vendor/model_mini".into()),
            ..Default::default()
        },
    )
    .expect("create");
    db.end_session("want", "done").expect("end");
    db.create_session(
        "other",
        "cli",
        &NewSession {
            model: Some("vendor/model-mini".into()),
            ..Default::default()
        },
    )
    .expect("create");
    db.end_session("other", "done").expect("end");

    let rows = db
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                model_like: Some("model_mini".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["want"]);
    db.close();
}

#[test]
fn cwd_prefix_underscore_is_literal_and_matches_children() {
    let (_dir, db) = open_db("state.db");
    mk(&db, "target", "cli", 60.0, None, 0, "done");
    update_cols(
        &db,
        "target",
        &[("cwd", Some("/home/me/my_project/src".into()))],
    );
    mk(&db, "sibling", "cli", 60.0, None, 0, "done");
    update_cols(
        &db,
        "sibling",
        &[("cwd", Some("/home/me/myXproject/src".into()))],
    );

    let rows = db
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                cwd_prefix: Some("/home/me/my_project".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["target"]);

    let pruned = db
        .prune_sessions(
            None,
            None,
            None,
            PruneFilters {
                cwd_prefix: Some("/home/me/my_project".into()),
                ..Default::default()
            },
        )
        .expect("prune");
    assert_eq!(pruned, 1);
    assert!(db.get_session("sibling").expect("get").is_some());

    // Control: the prefix matches the directory and its children.
    let (_dir2, db2) = open_db("state.db");
    mk(&db2, "root", "cli", 60.0, None, 0, "done");
    update_cols(&db2, "root", &[("cwd", Some("/home/me/proj".into()))]);
    mk(&db2, "child", "cli", 60.0, None, 0, "done");
    update_cols(&db2, "child", &[("cwd", Some("/home/me/proj/src".into()))]);
    mk(&db2, "outside", "cli", 60.0, None, 0, "done");
    update_cols(&db2, "outside", &[("cwd", Some("/home/me/other".into()))]);
    let rows = db2
        .list_prune_candidates(
            None,
            None,
            PruneFilters {
                cwd_prefix: Some("/home/me/proj".into()),
                ..Default::default()
            },
        )
        .expect("candidates");
    let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["child", "root"]);
    db2.close();
    db.close();
}

#[test]
fn cwd_prefix_percent_does_not_select_everything() {
    let (_dir, db) = open_db("state.db");
    mk(&db, "a", "cli", 60.0, None, 0, "done");
    update_cols(&db, "a", &[("cwd", Some("/home/me/one".into()))]);
    mk(&db, "b", "cli", 60.0, None, 0, "done");
    update_cols(&db, "b", &[("cwd", Some("/home/me/two".into()))]);

    assert_eq!(
        db.list_prune_candidates(
            None,
            None,
            PruneFilters {
                cwd_prefix: Some("/home/me/%".into()),
                ..Default::default()
            }
        )
        .expect("candidates")
        .len(),
        0
    );
    db.close();
}

// =====================================================================
// set_session_archived (compression lineage)
// =====================================================================

fn compression_pair(db: &SessionDB) {
    let base = 100.0;
    db.create_session("root", "cli", &NewSession::default())
        .expect("create");
    db.create_session(
        "tip",
        "cli",
        &NewSession {
            parent_session_id: Some("root".into()),
            ..Default::default()
        },
    )
    .expect("create");
    update_cols(
        db,
        "root",
        &[
            ("started_at", Some(format!("{}", now() - base))),
            ("ended_at", Some(format!("{}", now() - base + 10.0))),
            ("end_reason", Some("compression".into())),
            ("message_count", Some("1".into())),
        ],
    );
    update_cols(
        db,
        "tip",
        &[
            ("started_at", Some(format!("{}", now() - base + 20.0))),
            ("message_count", Some("1".into())),
        ],
    );
}

#[test]
fn archiving_compression_tip_archives_projected_root() {
    let (_dir, db) = open_db("state.db");
    compression_pair(&db);

    assert!(db.set_session_archived("tip", true).expect("archive"));

    assert_eq!(archived_flag(&db, "root"), Some(1));
    assert_eq!(archived_flag(&db, "tip"), Some(1));

    // Unarchive flips the whole lineage back.
    assert!(db.set_session_archived("tip", false).expect("unarchive"));
    assert_eq!(archived_flag(&db, "root"), Some(0));
    assert_eq!(archived_flag(&db, "tip"), Some(0));
    db.close();
}

#[test]
fn get_session_reports_archived_flag() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default())
        .expect("create");
    assert!(!db.get_session("s1").expect("get").expect("row").archived);
    db.set_session_archived("s1", true).expect("archive");
    assert!(db.get_session("s1").expect("get").expect("row").archived);
    db.close();
}

#[test]
fn archive_sessions_and_stale_sweep() {
    let (_dir, db) = open_db("state.db");
    mk(
        &db,
        "old-done",
        "cli",
        300.0 * 86400.0,
        Some("old"),
        0,
        "done",
    );
    mk(&db, "recent-done", "cli", 10.0, Some("recent"), 0, "done");
    // Not ended: archive_stale sweeps it, prune filters never do.
    db.create_session("open-old", "cli", &NewSession::default())
        .expect("create");
    update_cols(
        &db,
        "open-old",
        &[("started_at", Some(format!("{}", now() - 200.0 * 86400.0)))],
    );

    // archive_sessions (defaults archived=False; idle threshold).
    let n = db
        .archive_sessions(Some(30.0), None, PruneFilters::default())
        .expect("archive");
    assert_eq!(n, 1);
    assert_eq!(archived_flag(&db, "old-done"), Some(1));

    // Repeat is idempotent.
    let n = db
        .archive_sessions(Some(30.0), None, PruneFilters::default())
        .expect("archive");
    assert_eq!(n, 0);

    // archive_stale_sessions sweeps the still-open old session (pinned only
    // guards the Desktop keep flag; no pinned column writes here).
    let n = db.archive_stale_sessions(30.0, true).expect("stale");
    assert_eq!(n, 1);
    assert_eq!(archived_flag(&db, "open-old"), Some(1));
    // The recent session survives.
    assert_eq!(archived_flag(&db, "recent-done"), Some(0));
    db.close();
}

#[test]
fn prune_empty_ghost_sessions_removes_only_old_empty_tui() {
    let (_dir, db) = open_db("state.db");
    // Old, ended, empty, no title — the ghost.
    db.create_session("ghost", "tui", &NewSession::default())
        .expect("create");
    db.end_session("ghost", "done").expect("end");
    update_cols(
        &db,
        "ghost",
        &[("started_at", Some(format!("{}", now() - 48.0 * 86400.0)))],
    );
    // Recent empty tui — spared.
    db.create_session("recent-ghost", "tui", &NewSession::default())
        .expect("create");
    db.end_session("recent-ghost", "done").expect("end");
    update_cols(
        &db,
        "recent-ghost",
        &[("started_at", Some(format!("{}", now() - 3600.0)))],
    );
    // Old tui with a title — spared.
    db.create_session("titled", "tui", &NewSession::default())
        .expect("create");
    db.end_session("titled", "done").expect("end");
    update_cols(
        &db,
        "titled",
        &[
            ("started_at", Some(format!("{}", now() - 48.0 * 86400.0))),
            ("title", Some("kept".into())),
        ],
    );

    let n = db.prune_empty_ghost_sessions(None).expect("ghosts");
    assert_eq!(n, 1);
    assert!(db.get_session("ghost").expect("get").is_none());
    assert!(db.get_session("recent-ghost").expect("get").is_some());
    assert!(db.get_session("titled").expect("get").is_some());
    db.close();
}
