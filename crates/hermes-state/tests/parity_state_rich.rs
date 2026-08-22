//! Parity oracles for the surface read-helpers unit, mirroring upstream
//! tests/hermes_state/test_session_read_state.py, tests/test_hermes_state.py
//! (TestCounts, TestExcludeSources, TestCompressionChainProjection,
//! TestSessionPinAndStaleArchive, TestSessionIdSearch, gateway listing), and
//! hermes_state_search.py search_sessions_by_id @ b9aa928.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use hermes_state::activity::ActivityProvenance;
use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::rich::RichListParams;
use hermes_state::state::{now, SessionDB};
use serde_json::Value;

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

fn set_col(db: &SessionDB, table: &str, col: &str, val: &str, sid: &str) {
    let conn = db.writer_conn();
    if val == "NULL" {
        conn.execute(
            &format!("UPDATE {table} SET {col} = NULL WHERE {table}.id = ?"),
            rusqlite::params![sid],
        )
        .expect("update NULL");
    } else {
        conn.execute(
            &format!("UPDATE {table} SET {col} = ? WHERE {table}.id = ?"),
            rusqlite::params![val, sid],
        )
        .expect("update");
    }
}

fn set_ts(db: &SessionDB, table: &str, col: &str, ts: f64, sid: &str) {
    let conn = db.writer_conn();
    conn.execute(
        &format!("UPDATE {table} SET {col} = ? WHERE {table}.id = ?"),
        rusqlite::params![ts, sid],
    )
    .expect("update ts");
}

/// Key message updates by session_id (messages rows are keyed by row id).
fn set_msg_ts(db: &SessionDB, ts: f64, sid: &str) {
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE messages SET timestamp = ? WHERE session_id = ?",
        rusqlite::params![ts, sid],
    )
    .expect("update msg ts");
}

fn last_read(db: &SessionDB, sid: &str) -> Option<f64> {
    let conn = db.writer_conn();
    conn.query_row(
        "SELECT last_read_at FROM sessions WHERE id = ?",
        rusqlite::params![sid],
        |r| r.get(0),
    )
    .ok()
}

fn pinned_flag(db: &SessionDB, sid: &str) -> Option<i64> {
    let conn = db.writer_conn();
    conn.query_row(
        "SELECT pinned FROM sessions WHERE id = ?",
        rusqlite::params![sid],
        |r| r.get(0),
    )
    .ok()
}

fn rich_row(db: &SessionDB, sid: &str) -> Value {
    db.list_sessions_rich(&RichListParams {
        include_archived: true,
        ..Default::default()
    })
    .expect("rich")
    .into_iter()
    .find(|s| s.get("id").and_then(Value::as_str) == Some(sid))
    .unwrap_or_else(|| panic!("session {sid} not surfaced"))
}

fn ids(db: &SessionDB, params: &RichListParams) -> Vec<String> {
    db.list_sessions_rich(params)
        .expect("rich")
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect()
}

/// Build root -> delegate -> compression-child -> tip chain exactly like
/// upstream TestCompressionChainProjection._build_compression_chain.
fn build_compression_chain(db: &SessionDB, t0: f64) -> (String, String, String, String) {
    create(db, "root1", "cli");
    set_ts(db, "sessions", "started_at", t0, "root1");
    append(db, "root1", "user", "help me refactor auth");

    // Delegate subagent spawned while root1 was live (before it ended)
    db.create_session("delegate1", "cli", &NewSession {
        parent_session_id: Some("root1".into()),
        ..Default::default()
    })
    .expect("create delegate");
    set_ts(db, "sessions", "started_at", t0 + 600.0, "delegate1");
    set_ts(db, "sessions", "ended_at", t0 + 650.0, "delegate1");
    append(db, "delegate1", "user", "delegate task");

    // root1 compressed at t0+1800
    set_ts(db, "sessions", "ended_at", t0 + 1800.0, "root1");
    set_col(db, "sessions", "end_reason", "compression", "root1");

    // Continuation mid created 1s after parent ended
    db.create_session("mid1", "cli", &NewSession {
        parent_session_id: Some("root1".into()),
        ..Default::default()
    })
    .expect("create mid");
    set_ts(db, "sessions", "started_at", t0 + 1801.0, "mid1");
    append(db, "mid1", "user", "continuing");

    // mid1 also compressed
    set_ts(db, "sessions", "ended_at", t0 + 2700.0, "mid1");
    set_col(db, "sessions", "end_reason", "compression", "mid1");

    // Tip — latest continuation
    db.create_session("tip1", "cli", &NewSession {
        parent_session_id: Some("mid1".into()),
        ..Default::default()
    })
    .expect("create tip");
    set_ts(db, "sessions", "started_at", t0 + 2701.0, "tip1");
    append(db, "tip1", "user", "latest message");
    ("root1".into(), "delegate1".into(), "mid1".into(), "tip1".into())
}

/// Compression pair: root ended via compression, tip started after.
fn make_compression_pair(db: &SessionDB) {
    let base = now() - 100.0;
    create(db, "root", "cli");
    db.create_session("tip", "cli", &NewSession {
        parent_session_id: Some("root".into()),
        ..Default::default()
    })
    .expect("create tip");
    set_ts(db, "sessions", "started_at", base, "root");
    set_ts(db, "sessions", "ended_at", base + 10.0, "root");
    set_col(db, "sessions", "end_reason", "compression", "root");
    set_col(db, "sessions", "message_count", "1", "root");
    set_ts(db, "sessions", "started_at", base + 20.0, "tip");
    set_col(db, "sessions", "message_count", "1", "tip");
}

/// A session whose latest activity was `days_idle` days ago.
fn make_idle(db: &SessionDB, sid: &str, days_idle: f64, source: &str) {
    create(db, sid, source);
    append(db, sid, "user", &format!("msg {sid}"));
    let old = now() - days_idle * 86400.0;
    set_ts(db, "sessions", "started_at", old, sid);
    set_ts(db, "messages", "timestamp", old, sid);
}

// =====================================================================
// tests/hermes_state/test_session_read_state.py — read watermarks
// =====================================================================

#[test]
fn untracked_sessions_are_read() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "hi");

    assert_eq!(last_read(&db, "s1"), None); // NULL watermark
    assert_eq!(rich_row(&db, "s1")["unread"], Value::Bool(false));
}

#[test]
fn mark_read_then_new_activity_flips_back_to_unread() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "hi");

    assert!(db.set_session_read("s1", true).expect("read"));
    assert_eq!(rich_row(&db, "s1")["unread"], Value::Bool(false));

    // New activity postdating the watermark makes it unread again without
    // any write on the message path.
    thread::sleep(Duration::from_millis(10));
    append(&db, "s1", "assistant", "reply");
    assert_eq!(rich_row(&db, "s1")["unread"], Value::Bool(true));
}

#[test]
fn mark_unread_explicitly() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    append(&db, "s1", "user", "hi");
    db.set_session_read("s1", true).expect("read");

    assert!(db.set_session_read("s1", false).expect("unread"));
    assert_eq!(last_read(&db, "s1"), Some(0.0));
    assert_eq!(rich_row(&db, "s1")["unread"], Value::Bool(true));
}

#[test]
fn missing_session_returns_false() {
    let (_dir, db) = open_db("state.db");
    assert!(!db.set_session_read("nope", true).expect("missing"));
    assert!(!db.set_session_read("nope", false).expect("missing"));
}

#[test]
fn reading_compression_tip_stamps_whole_lineage() {
    let (_dir, db) = open_db("state.db");
    make_compression_pair(&db);

    assert!(db.set_session_read("tip", true).expect("read tip"));

    let root_read = last_read(&db, "root");
    assert!(root_read.is_some() && root_read.unwrap() > 0.0);
    assert_eq!(root_read, last_read(&db, "tip"));

    // The projected conversation row (root surfaced as tip) derives read.
    let rows = db
        .list_sessions_rich(&RichListParams {
            order_by_last_active: true,
            ..Default::default()
        })
        .expect("rich");
    let surfaced: Vec<String> = rows
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect();
    assert_eq!(surfaced, vec!["tip".to_string()]);
    assert_eq!(rows[0]["unread"], Value::Bool(false));
}

#[test]
fn marking_root_unread_marks_projected_conversation() {
    let (_dir, db) = open_db("state.db");
    make_compression_pair(&db);
    db.set_session_read("tip", true).expect("read tip");

    assert!(db.set_session_read("root", false).expect("unread root"));

    let rows = db
        .list_sessions_rich(&RichListParams {
            order_by_last_active: true,
            ..Default::default()
        })
        .expect("rich");
    let surfaced: Vec<String> = rows
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect();
    assert_eq!(surfaced, vec!["tip".to_string()]);
    assert_eq!(rows[0]["unread"], Value::Bool(true));
}

// =====================================================================
// tests/test_hermes_state.py TestCounts — session counts
// =====================================================================

#[test]
fn session_count_by_source() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    create(&db, "s2", "telegram");
    create(&db, "s3", "cli");
    assert_eq!(
        db.session_count(Some("cli"), &[], None, 0, false, false, false, &[])
            .expect("count cli"),
        2
    );
    assert_eq!(
        db.session_count(Some("telegram"), &[], None, 0, false, false, false, &[])
            .expect("count telegram"),
        1
    );
}

#[test]
fn session_count_ge_empty() {
    let (_dir, db) = open_db("state.db");
    assert!(!db.session_count_ge(1).expect("ge1"));
    assert!(!db.session_count_ge(2).expect("ge2"));
}

#[test]
fn session_count_ge_at_threshold() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    assert!(db.session_count_ge(1).expect("ge1"));
    assert!(!db.session_count_ge(2).expect("ge2"));

    create(&db, "s2", "telegram");
    assert!(db.session_count_ge(1).expect("ge1"));
    assert!(db.session_count_ge(2).expect("ge2"));
    assert!(!db.session_count_ge(3).expect("ge3"));
}

#[test]
fn session_count_by_source_grouping_and_exclude_children() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    db.create_session("c1", "cli", &NewSession {
        parent_session_id: Some("s1".into()),
        ..Default::default()
    })
    .expect("child");
    let by_source = db
        .session_count_by_source(false, false, false)
        .expect("by source");
    assert_eq!(by_source.get("cli").copied(), Some(2));
    // exclude_children mirrors list_sessions_rich visibility.
    let surfaced = db
        .session_count_by_source(false, false, true)
        .expect("by source excl");
    assert_eq!(surfaced.get("cli").copied(), Some(1));
    let total = db
        .session_count(None, &[], None, 0, true, false, true, &[])
        .expect("count excl children");
    assert_eq!(total, 1);
}

#[test]
fn count_empty_sessions_filters_live_and_archived() {
    let (_dir, db) = open_db("state.db");
    // Empty + ended + not archived → counted.
    create(&db, "empty_ended", "cli");
    db.end_session("empty_ended", "tui_shutdown").expect("end");
    assert_eq!(db.count_empty_sessions().expect("count"), 1);
    // Archived empty → not counted.
    db.set_session_archived("empty_ended", true).expect("archive");
    assert_eq!(db.count_empty_sessions().expect("count after archive"), 0);
}

// =====================================================================
// tests/test_hermes_state.py — list_gateway_sessions + activity heartbeat
// =====================================================================

#[test]
fn list_gateway_sessions_newest_row_per_key_and_activity_heartbeat() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "gw-1",
        "telegram",
        &NewSession {
            session_key: Some("agent:main:telegram:dm:c1".into()),
            chat_id: Some("c1".into()),
            chat_type: Some("dm".into()),
            ..Default::default()
        },
    )
    .expect("create gw");
    append(&db, "gw-1", "user", "ping");
    set_msg_ts(&db, 1_700_000_000.0, "gw-1");

    let heartbeat = 1_700_000_900.0;
    db.touch_session_activity("gw-1", Some(heartbeat), Some("compressing context"), None)
        .expect("touch");
    let rows = db.list_gateway_sessions(None, true).expect("gateway rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["last_active"].as_f64(), Some(heartbeat));
    let activity = db.get_session_activity("gw-1").expect("activity").expect("some");
    let o = activity.as_object().expect("obj");
    assert_eq!(o["last_activity_description"], Value::String("compressing context".into()));
    assert_eq!(o["last_activity_ts"].as_f64(), Some(heartbeat));
}

#[test]
fn gateway_sessions_resolve_newest_row_per_session_key() {
    let (_dir, db) = open_db("state.db");
    let key = "agent:main:telegram:dm:lane";
    let base = NewSession {
        session_key: Some(key.into()),
        chat_id: Some("lane".into()),
        chat_type: Some("dm".into()),
        user_id: Some("lane-user".into()),
        ..Default::default()
    };
    db.create_session("old", "telegram", &base).expect("old");
    db.create_session("new", "telegram", &base).expect("new");
    db.create_session("other", "telegram", &NewSession {
        session_key: Some("other-key".into()),
        ..Default::default()
    })
    .expect("other");

    let rows = db.list_gateway_sessions(None, false).expect("rows");
    let gw: Vec<String> = rows
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect();
    assert!(gw.contains(&"new".to_string()));
    assert!(!gw.contains(&"old".to_string()));
    assert!(gw.contains(&"other".to_string()));
}

// =====================================================================
// tests/test_hermes_state.py — rich list filters / session_key / search
// =====================================================================

#[test]
fn rich_list_session_key_filter_precedes_limit() {
    let (_dir, db) = open_db("state.db");
    let lane_key = "agent:main:telegram:dm:lane";
    let lane = |_sid: &str| NewSession {
        session_key: Some(lane_key.into()),
        user_id: Some("lane-user".into()),
        chat_id: Some("lane".into()),
        ..Default::default()
    };
    db.create_session("lane_oldest", "telegram", &lane("lane_oldest")).expect("oldest");
    db.create_session("lane_newest", "telegram", &lane("lane_newest")).expect("newest");
    for i in 0..60 {
        let key = format!("agent:main:telegram:dm:foreign-{i}");
        db.create_session(
            &format!("foreign_{i}"),
            "telegram",
            &NewSession {
                session_key: Some(key),
                user_id: Some(format!("foreign-user-{i}")),
                chat_id: Some(format!("foreign-{i}")),
                ..Default::default()
            },
        )
        .expect("foreign");
    }
    db.create_session("legacy_null_key", "telegram", &NewSession {
        user_id: Some("lane-user".into()),
        chat_id: Some("lane".into()),
        ..Default::default()
    })
    .expect("legacy");

    let sessions = ids(
        &db,
        &RichListParams {
            source: Some("telegram".into()),
            session_key: Some(lane_key.into()),
            limit: 2,
            ..Default::default()
        },
    );
    assert_eq!(sessions, vec!["lane_newest".to_string(), "lane_oldest".to_string()]);
}

#[test]
fn rich_list_session_key_scopes_search_and_projects_compression() {
    let (_dir, db) = open_db("state.db");
    let lane_key = "agent:main:telegram:dm:lane";
    let lane = |_sid: &str| NewSession {
        session_key: Some(lane_key.into()),
        user_id: Some("lane-user".into()),
        chat_id: Some("lane".into()),
        ..Default::default()
    };
    db.create_session("lane_root", "telegram", &lane("lane_root")).expect("root");
    db.set_session_title("lane_root", "Needle root").expect("title root");
    db.end_session("lane_root", "compression").expect("end root");
    db.create_session("lane_tip", "telegram", &NewSession {
        parent_session_id: Some("lane_root".into()),
        session_key: Some(lane_key.into()),
        user_id: Some("lane-user".into()),
        chat_id: Some("lane".into()),
        ..Default::default()
    })
    .expect("tip");
    db.set_session_title("lane_tip", "Needle continuation").expect("title tip");
    append(&db, "lane_tip", "user", "latest lane activity");
    db.create_session("foreign_match", "telegram", &NewSession {
        session_key: Some("agent:main:telegram:dm:foreign".into()),
        user_id: Some("foreign-user".into()),
        chat_id: Some("foreign".into()),
        ..Default::default()
    })
    .expect("foreign");
    db.set_session_title("foreign_match", "Needle foreign").expect("title foreign");

    let rows = db
        .list_sessions_rich(&RichListParams {
            source: Some("telegram".into()),
            session_key: Some(lane_key.into()),
            search_query: Some("needle".into()),
            order_by_last_active: true,
            limit: 1,
            ..Default::default()
        })
        .expect("rich");
    let surfaced: Vec<String> = rows
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect();
    assert_eq!(surfaced, vec!["lane_tip".to_string()]);
    assert_eq!(
        rows[0].get("_lineage_root_id").and_then(Value::as_str),
        Some("lane_root")
    );
}

// =====================================================================
// tests/test_hermes_state.py — compression chain projection
// =====================================================================

#[test]
fn get_compression_tip_walks_full_chain() {
    let (_dir, db) = open_db("state.db");
    build_compression_chain(&db, now() - 3600.0);
    assert_eq!(db.get_compression_tip("root1").expect("tip root"), "tip1");
    assert_eq!(db.get_compression_tip("mid1").expect("tip mid"), "tip1");
    assert_eq!(db.get_compression_tip("tip1").expect("tip tip"), "tip1");
}

#[test]
fn subagent_session_still_hidden() {
    let (_dir, db) = open_db("state.db");
    create(&db, "root", "cli");
    db.create_session("delegate", "cli", &NewSession {
        parent_session_id: Some("root".into()),
        ..Default::default()
    })
    .expect("delegate");

    let sessions = ids(&db, &RichListParams::default());
    assert!(!sessions.contains(&"delegate".to_string()));
    assert!(sessions.contains(&"root".to_string()));
}

#[test]
fn list_surfaces_tip_for_compressed_root() {
    let (_dir, db) = open_db("state.db");
    build_compression_chain(&db, now() - 3600.0);
    // Add an uncompressed root for comparison.
    create(&db, "solo", "cli");
    append(&db, "solo", "user", "standalone");

    let sessions = db
        .list_sessions_rich(&RichListParams {
            source: Some("cli".into()),
            limit: 20,
            ..Default::default()
        })
        .expect("rich");
    let surfaced: Vec<String> = sessions
        .iter()
        .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
        .collect();
    assert!(surfaced.contains(&"tip1".to_string()));
    assert!(surfaced.contains(&"solo".to_string()));
    assert!(!surfaced.contains(&"root1".to_string()));
    assert!(!surfaced.contains(&"mid1".to_string()));
    assert!(!surfaced.contains(&"delegate1".to_string()));

    let tip_row = sessions
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some("tip1"))
        .expect("tip row");
    assert_eq!(
        tip_row.get("_lineage_root_id").and_then(Value::as_str),
        Some("root1")
    );
    assert!(tip_row["preview"].as_str().unwrap_or("").starts_with("latest message"));
    assert_eq!(tip_row.get("ended_at"), Some(&Value::Null));
    assert_eq!(tip_row.get("end_reason"), Some(&Value::Null));
}

#[test]
fn list_projects_multiple_independent_chains_in_one_call() {
    let (_dir, db) = open_db("state.db");
    let t0 = now() - 7200.0;
    build_compression_chain(&db, t0);
    // Second, independent chain — same shape, different ids/content.
    create(&db, "root2", "cli");
    set_ts(&db, "sessions", "started_at", t0 + 100.0, "root2");
    append(&db, "root2", "user", "second conversation start");
    set_ts(&db, "sessions", "ended_at", t0 + 1900.0, "root2");
    set_col(&db, "sessions", "end_reason", "compression", "root2");
    db.create_session("tip2", "cli", &NewSession {
        parent_session_id: Some("root2".into()),
        ..Default::default()
    })
    .expect("tip2");
    set_ts(&db, "sessions", "started_at", t0 + 1901.0, "tip2");
    append(&db, "tip2", "user", "second chain live tip");

    let sessions = db
        .list_sessions_rich(&RichListParams {
            source: Some("cli".into()),
            limit: 20,
            ..Default::default()
        })
        .expect("rich");
    let by_id: std::collections::HashMap<&str, &Value> = sessions
        .iter()
        .filter_map(|s| {
            s.get("id")
                .and_then(Value::as_str)
                .map(|id| (id, s))
        })
        .collect();
    assert!(by_id.contains_key("tip1"));
    assert!(by_id.contains_key("tip2"));
    assert_eq!(
        by_id["tip1"].get("_lineage_root_id").and_then(Value::as_str),
        Some("root1")
    );
    assert_eq!(
        by_id["tip2"].get("_lineage_root_id").and_then(Value::as_str),
        Some("root2")
    );
    assert!(by_id["tip1"]["preview"].as_str().unwrap_or("").starts_with("latest message"));
    assert!(by_id["tip2"]["preview"].as_str().unwrap_or("").starts_with("second chain live tip"));
}

// =====================================================================
// tests/test_hermes_state.py TestExcludeSources
// =====================================================================

#[test]
fn list_sessions_rich_excludes_tool_source() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    create(&db, "s2", "tool");
    create(&db, "s3", "telegram");
    let sessions = ids(
        &db,
        &RichListParams {
            exclude_sources: vec!["tool".into()],
            ..Default::default()
        },
    );
    assert!(sessions.contains(&"s1".to_string()));
    assert!(sessions.contains(&"s3".to_string()));
    assert!(!sessions.contains(&"s2".to_string()));
}

// =====================================================================
// tests/test_hermes_state.py TestSessionPinAndStaleArchive — pin flag
// =====================================================================

#[test]
fn set_session_pinned_roundtrip() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    assert!(db.set_session_pinned("s1", true).expect("pin"));
    assert_eq!(pinned_flag(&db, "s1"), Some(1));
    assert!(db.set_session_pinned("s1", false).expect("unpin"));
    assert_eq!(pinned_flag(&db, "s1"), Some(0));
}

#[test]
fn pinned_session_survives_the_limit_window() {
    let (_dir, db) = open_db("state.db");
    for i in 0..6 {
        make_idle(&db, &format!("s{i}"), (6 - i) as f64, "cli");
    }
    db.set_session_pinned("s0", true).expect("pin s0");

    let page = ids(
        &db,
        &RichListParams {
            limit: 3,
            min_message_count: 1,
            order_by_last_active: true,
            ..Default::default()
        },
    );
    assert!(!page.contains(&"s0".to_string()), "precondition: pin off the page");

    let with_pins = ids(
        &db,
        &RichListParams {
            limit: 3,
            min_message_count: 1,
            order_by_last_active: true,
            include_pinned: true,
            ..Default::default()
        },
    );
    assert!(with_pins.contains(&"s0".to_string()));
    assert_eq!(with_pins[..3], page[..]);
    assert_eq!(with_pins.len(), page.len() + 1);
}

// =====================================================================
// tests/test_hermes_state.py TestSessionIdSearch + activity provenance
// =====================================================================

#[test]
fn search_sessions_by_id_matches_exact_prefix_and_substring() {
    let (_dir, db) = open_db("state.db");
    for (sid, content) in [
        ("20260603_090200_abcd12", "content without id"),
        ("20260602_111111_other99", "other content"),
    ] {
        let (sid, content) = (sid.to_string(), content.to_string());
        db.create_session(&sid, "cli", &NewSession {
            model: Some("test-model".into()),
            ..Default::default()
        })
        .expect("seed");
        append(&db, &sid, "user", &content);
    }

    let hit = |q: &str| -> Vec<String> {
        db.search_sessions_by_id(q, 20, true, None, Vec::new(), Vec::new())
            .expect("search")
            .iter()
            .filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string()))
            .collect()
    };
    assert_eq!(hit("20260603_090200_abcd12"), vec!["20260603_090200_abcd12".to_string()]);
    assert_eq!(hit("20260603"), vec!["20260603_090200_abcd12".to_string()]);
    assert_eq!(hit("ABCD12"), vec!["20260603_090200_abcd12".to_string()]);
    assert!(hit("").is_empty());
}

#[test]
fn activity_provenance_roundtrip_and_clear_labels() {
    let (_dir, db) = open_db("state.db");
    create(&db, "s1", "cli");
    let before = db.get_session_activity("s1").expect("activity before").expect("some");
    assert_eq!(before["last_activity_at"], Value::Null);
    assert_eq!(before["provenance"], Value::String("unknown".into()));

    db.touch_session_activity(
        "s1",
        Some(1_700_000_500.0),
        Some("executing tool"),
        Some(&ActivityProvenance::AgentCompression),
    )
    .expect("touch");
    let snap = db.get_session_activity("s1").expect("activity").expect("some").as_object().unwrap().clone();
    assert_eq!(snap["last_activity_provenance"], Value::String("agent.compression".into()));
    assert_eq!(snap["last_activity_description"], Value::String("executing tool".into()));

    // Observed-only: an older timestamp never moves the watermark back.
    db.touch_session_activity("s1", Some(1_000_000.0), Some("stale"), None)
        .expect("stale touch");
    let snap = db.get_session_activity("s1").expect("activity2").expect("some");
    assert_eq!(snap["description"], Value::String("executing tool".into()));

    db.clear_session_activity_labels("s1").expect("clear");
    let snap = db.get_session_activity("s1").expect("activity3").expect("some").as_object().unwrap().clone();
    assert_eq!(snap["description"], Value::String("".into()));
    assert_eq!(snap["provenance"], Value::String("unknown".into()));
    // Timestamp survives the label clear.
    assert_eq!(snap["last_activity_at"].as_f64(), Some(1_700_000_500.0));
}
