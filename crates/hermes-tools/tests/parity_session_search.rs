//! Parity oracles for the session_search tool shapes, mirroring upstream
//! tests/tools/test_session_search.py @ b9aa928 (browse/discover/scroll/
//! read core; the cross-profile tests are deferred with the profiles crate).


use hermes_tools::registry::registry;
use hermes_tools::session_search::{register_session_search, session_search};
use hermes_state::crud::{MessageInput, NewSession};
use hermes_state::state::{now, SessionDB};
use serde_json::{json, Value};

fn open_db(name: &str) -> (tempfile::TempDir, SessionDB) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    let db = SessionDB::open(Some(path), false).expect("open");
    (dir, db)
}

fn msg(role: &str, content: &str) -> MessageInput {
    MessageInput {
        role: role.to_string(),
        content: Some(json!(content)),
        ..Default::default()
    }
}

fn seed_modpack_sessions(db: &SessionDB) {
    let now_ts = now();
    db.create_session("s_oldest", "cli", &NewSession::default()).expect("oldest");
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET started_at = ?, title = ? WHERE id = ?",
        rusqlite::params![now_ts - 30000.0, "Building the Modpack", "s_oldest"],
    )
    .expect("meta oldest");
    drop(conn);
    db.append_message("s_oldest", &msg("user", "Let's build a Minecraft modpack"), None).expect("m1");
    db.append_message("s_oldest", &msg("assistant", "Great. Let me scaffold the modpack repo."), None).expect("m2");
    db.append_message("s_oldest", &msg("user", "Use NeoForge 1.21.1"), None).expect("m3");
    db.append_message("s_oldest", &msg("assistant", "Done. Modpack repo created with NeoForge 1.21.1."), None).expect("m4");
    db.append_message("s_oldest", &msg("assistant", "Tier-0 mods installed; modpack smoke test passes."), None).expect("m5");

    db.create_session("s_middle", "cli", &NewSession::default()).expect("middle");
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET started_at = ?, title = ? WHERE id = ?",
        rusqlite::params![now_ts - 15000.0, "Modpack Quest Coverage", "s_middle"],
    )
    .expect("meta middle");
    drop(conn);
    db.append_message("s_middle", &msg("user", "Deep-dive every modpack reference quest guide"), None).expect("n1");
    db.append_message("s_middle", &msg("assistant", "Surveying ATM10 questbook for modpack inspiration."), None).expect("n2");
    db.append_message("s_middle", &msg("user", "Update the modpack version too"), None).expect("n3");
    db.append_message("s_middle", &msg("assistant", "Modpack version bumped 0.4 → 0.8.5; quest coverage page added."), None).expect("n4");

    db.create_session("s_newest", "cli", &NewSession::default()).expect("newest");
    let conn = db.writer_conn();
    conn.execute(
        "UPDATE sessions SET started_at = ?, title = ? WHERE id = ?",
        rusqlite::params![now_ts - 1000.0, "Modpack Mob Spawn Fix", "s_newest"],
    )
    .expect("meta newest");
    drop(conn);
    db.append_message("s_newest", &msg("user", "Fix the modpack mob spawning"), None).expect("o1");
    db.append_message("s_newest", &msg("assistant", "Investigating elite mob gating in the modpack KubeJS."), None).expect("o2");
    db.append_message("s_newest", &msg("assistant", "Shipped commit b850442. Modpack alternator nerfed too."), None).expect("o3");
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).expect("json")
}

#[test]
fn browse_returns_recent_sessions_and_excludes_current() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let result = parse(&session_search(Some(&db), "", None, 3, None, None, 5, None, None, None));
    assert_eq!(result["success"], json!(true));
    assert_eq!(result["mode"], json!("browse"));
    assert!(result["count"].as_i64().unwrap() >= 3);

    let result = parse(&session_search(
        Some(&db), "", None, 3, None, None, 5, None, None, Some("s_newest"),
    ));
    let sids: Vec<&str> = result["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["session_id"].as_str())
        .collect();
    assert!(!sids.contains(&"s_newest"));
}

#[test]
fn discovery_returns_bookends_and_window() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let result = parse(&session_search(Some(&db), "modpack", None, 3, None, None, 5, None, None, None));
    assert_eq!(result["success"], json!(true));
    assert_eq!(result["mode"], json!("discover"));
    assert!(result["count"].as_i64().unwrap() >= 1);
    for hit in result["results"].as_array().unwrap() {
        assert!(hit.get("bookend_start").is_some());
        assert!(hit.get("messages").is_some());
        assert!(hit.get("bookend_end").is_some());
        assert!(hit.get("match_message_id").is_some());
        assert!(hit.get("snippet").is_some());
        assert!(hit.get("messages_before").is_some());
        assert!(hit.get("messages_after").is_some());
    }
}

#[test]
fn discovery_filters_current_session() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let result = parse(&session_search(Some(&db), "modpack", None, 3, None, None, 5, None, None, Some("s_newest")));
    let sids: Vec<&str> = result["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["session_id"].as_str())
        .collect();
    assert!(!sids.contains(&"s_newest"));
}

#[test]
fn discovery_sort_newest_and_oldest() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let newest = parse(&session_search(Some(&db), "modpack", None, 3, None, None, 5, Some("newest"), None, None));
    let first = &newest["results"][0];
    let sid = first["session_id"].as_str().unwrap();
    assert!(sid == "s_newest" || first.get("title").and_then(Value::as_str).unwrap_or("").contains("Newest"));

    let oldest = parse(&session_search(Some(&db), "modpack", None, 3, None, None, 5, Some("oldest"), None, None));
    assert_eq!(oldest["results"][0]["session_id"].as_str(), Some("s_oldest"));
}

#[test]
fn scroll_returns_anchored_window_without_bookends() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let disc = parse(&session_search(Some(&db), "modpack", None, 1, None, None, 5, None, None, None));
    let anchor_sid = disc["results"][0]["session_id"].as_str().unwrap().to_string();
    let anchor_mid = disc["results"][0]["match_message_id"].as_i64().unwrap();

    let result = parse(&session_search(
        Some(&db), "", None, 3, Some(&anchor_sid), Some(anchor_mid), 2, None, None, None,
    ));
    assert_eq!(result["success"], json!(true));
    assert_eq!(result["mode"], json!("scroll"));
    assert!(result.get("bookend_start").is_none());
    assert!(result.get("bookend_end").is_none());
    let anchor_in_window: Vec<&Value> = result["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["id"].as_i64() == Some(anchor_mid))
        .collect();
    assert_eq!(anchor_in_window.len(), 1);
    assert_eq!(anchor_in_window[0]["anchor"].as_bool(), Some(true));
}

#[test]
fn scroll_window_clamped_to_20() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let disc = parse(&session_search(Some(&db), "modpack", None, 1, None, None, 5, None, None, None));
    let anchor_sid = disc["results"][0]["session_id"].as_str().unwrap().to_string();
    let anchor_mid = disc["results"][0]["match_message_id"].as_i64().unwrap();
    let result = parse(&session_search(
        Some(&db), "", None, 3, Some(&anchor_sid), Some(anchor_mid), 999, None, None, None,
    ));
    assert_eq!(result["window"], json!(20));
}

#[test]
fn read_returns_full_session() {
    let (_dir, db) = open_db("state.db");
    seed_modpack_sessions(&db);
    let result = parse(&session_search(Some(&db), "", None, 3, Some("s_oldest"), None, 5, None, None, None));
    assert_eq!(result["success"], json!(true));
    assert_eq!(result["mode"], json!("read"));
    assert_eq!(result["message_count"], json!(5));
    assert_eq!(result["messages"].as_array().unwrap().len(), 5);
    assert_eq!(result["session_meta"]["title"].as_str(), Some("Building the Modpack"));
}

#[test]
fn read_missing_session_returns_error() {
    let (_dir, db) = open_db("state.db");
    let result = parse(&session_search(Some(&db), "", None, 3, Some("nope"), None, 5, None, None, None));
    assert!(result.get("error").is_some());
}

#[test]
fn schema_params_cover_every_shape() {
    use hermes_tools::session_search::SESSION_SEARCH_SCHEMA;
    let params = SESSION_SEARCH_SCHEMA["parameters"]["properties"].as_object().unwrap();
    assert!(params.contains_key("query"));
    assert!(params.contains_key("limit"));
    assert_eq!(SESSION_SEARCH_SCHEMA["parameters"]["properties"]["sort"]["enum"], json!(["newest", "oldest"]));
    assert!(params.contains_key("session_id"));
    assert!(params.contains_key("around_message_id"));
    assert!(params.contains_key("window"));
    assert!(params.contains_key("role_filter"));
}

#[test]
fn registry_registers_session_search() {
    register_session_search();
    let reg = registry();
    assert_eq!(reg.get_toolset_for_tool("session_search").as_deref(), Some("session_search"));
    assert_eq!(reg.get_emoji("session_search", "⚡"), "🔍");
    let defs = reg.get_definitions(&std::collections::HashSet::from(["session_search".to_string()]), false);
    assert_eq!(defs.len(), 1);
}
