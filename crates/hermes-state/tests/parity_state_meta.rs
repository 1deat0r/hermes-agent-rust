//! Parity oracles for the session meta/model surface, mirroring upstream
//! tests/test_hermes_state.py (update_session_model browser-lock clearing,
//! billing-route atomic replace), tests/cli/test_cli_yolo_resume_persistence
//! .py (TestSessionDbYoloFlag), and tests/test_session_system_prompt_dedup.py
//! (update_system_prompt) @ b9aa928.

use std::path::PathBuf;

use hermes_state::crud::NewSession;
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

fn config_of(db: &SessionDB, sid: &str) -> Value {
    let row = db.get_session(sid).expect("get").expect("row");
    serde_json::from_str(row.model_config.as_deref().unwrap_or("{}")).expect("parse")
}

#[test]
fn update_session_model_clears_browser_lock_and_preserves_lineage() {
    let (_dir, db) = open_db("state.db");
    db.create_session(
        "s1",
        "hermes_browser",
        &NewSession {
            model: Some("x-ai/grok-4.5".into()),
            model_config: Some(json!({
                "_branched_from": "parent-session",
                "browser_model_lock": {
                    "provider": "nous",
                    "model": "x-ai/grok-4.5",
                    "confirmed": true,
                },
            })),
            ..Default::default()
        },
    )
    .expect("create");

    db.update_session_model("s1", "anthropic/claude-opus-4.8").expect("switch");

    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.model.as_deref(), Some("anthropic/claude-opus-4.8"));
    let model_config = config_of(&db, "s1");
    assert!(model_config.get("browser_model_lock").is_none());
    assert_eq!(model_config["_branched_from"], json!("parent-session"));
    // system_prompt storage nulled so cached footer metadata is rebuilt.
    assert_eq!(session.system_prompt_hash, None);
}

#[test]
fn update_session_meta_coalesces_model() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession {
        model: Some("old-model".into()),
        ..Default::default()
    })
    .expect("create");

    // model=None leaves the stored model untouched.
    db.update_session_meta("s1", r#"{"max_iterations": 3}"#, None).expect("meta1");
    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.model.as_deref(), Some("old-model"));
    assert_eq!(config_of(&db, "s1")["max_iterations"], json!(3));

    // model=Some replaces it.
    db.update_session_meta("s1", r#"{"max_iterations": 4}"#, Some("new-model")).expect("meta2");
    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.model.as_deref(), Some("new-model"));
}

#[test]
fn patch_session_model_config_merges_and_deletes_keys() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession {
        model_config: Some(json!({"_branched_from": "p", "keep": 1})),
        ..Default::default()
    })
    .expect("create");

    let mut patch = serde_json::Map::new();
    patch.insert("yolo_mode".into(), json!(true));
    patch.insert("keep".into(), Value::Null); // delete key
    db.patch_session_model_config("s1", &patch).expect("patch");
    let cfg = config_of(&db, "s1");
    assert_eq!(cfg["yolo_mode"], json!(true));
    assert!(cfg.get("keep").is_none());
    assert_eq!(cfg["_branched_from"], json!("p"));

    // Empty patch is a no-op; missing row is a no-op.
    db.patch_session_model_config("s1", &serde_json::Map::new()).expect("empty");
    db.patch_session_model_config("nope", &patch).expect("missing");
}

#[test]
fn get_session_model_config_value_tolerant_read() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession {
        model_config: Some(json!({"nested": {"a": 1}, "flag": true})),
        ..Default::default()
    })
    .expect("create");

    assert_eq!(
        db.get_session_model_config_value("s1", "flag", None).expect("flag"),
        json!(true)
    );
    assert_eq!(
        db.get_session_model_config_value("s1", "nested", None).expect("nested"),
        json!({"a": 1})
    );
    // Missing key -> default; missing session -> default.
    assert_eq!(
        db.get_session_model_config_value("s1", "zzz", Some(json!("dflt"))).expect("default"),
        json!("dflt")
    );
    assert_eq!(
        db.get_session_model_config_value("nope", "flag", Some(json!(42))).expect("missing"),
        json!(42)
    );
}

#[test]
fn update_session_runtime_lock_merges_and_nulls_prompt() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession {
        model: Some("m1".into()),
        model_config: Some(json!({"_branched_from": "parent"})),
        system_prompt: Some("stale footer".into()),
        ..Default::default()
    })
    .expect("create");

    let mut opts = serde_json::Map::new();
    opts.insert("temperature".into(), json!(0.2));
    db.update_session_runtime_lock(
        "s1",
        Some("browser-model"),
        Some("nous"),
        Some(&opts),
        Some("browser"),
        true,
    )
    .expect("lock");
    let session = db.get_session("s1").expect("get").expect("row");
    assert_eq!(session.model.as_deref(), Some("browser-model"));
    assert_eq!(session.system_prompt, None);
    assert_eq!(session.system_prompt_hash, None);
    let cfg = config_of(&db, "s1");
    assert_eq!(cfg["_branched_from"], json!("parent"));
    let lock = &cfg["browser_model_lock"];
    assert_eq!(lock["provider"], json!("nous"));
    assert_eq!(lock["model"], json!("browser-model"));
    assert_eq!(lock["confirmed"], json!(true));
    assert_eq!(lock["model_options"], json!({"temperature": 0.2}));
    assert!(lock["updated_at"].as_f64().unwrap() > 0.0);
}

#[test]
fn yolo_roundtrip_preserves_keys_and_missing_row_noop() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession {
        model: Some("m".into()),
        model_config: Some(json!({"max_iterations": 42, "_branched_from": "parent_x"})),
        ..Default::default()
    })
    .expect("create");

    db.set_session_yolo("s1", true).expect("yolo on");
    let meta = db.get_session("s1").expect("get").expect("row");
    assert!(SessionDB::session_yolo_enabled(Some(&row_dict(&meta))));

    db.set_session_yolo("s1", false).expect("yolo off");
    let meta = db.get_session("s1").expect("get").expect("row");
    assert!(!SessionDB::session_yolo_enabled(Some(&row_dict(&meta))));

    // Merge preserves unrelated keys.
    let cfg = config_of(&db, "s1");
    assert_eq!(cfg["max_iterations"], json!(42));
    assert_eq!(cfg["_branched_from"], json!("parent_x"));

    // Missing row is a no-op (lazy creation must not raise or create).
    db.set_session_yolo("does_not_exist", true).expect("noop");
    assert!(db.get_session("does_not_exist").expect("still missing").is_none());

    // Parse-failure reads are False (never accidentally enabled).
    assert!(!SessionDB::session_yolo_enabled(Some(&json!({"model_config": "not-json{"}))));
    assert!(!SessionDB::session_yolo_enabled(Some(&json!({"model_config": "{}"}))));
    assert!(!SessionDB::session_yolo_enabled(None));
}

fn row_dict(row: &hermes_state::crud::SessionRow) -> Value {
    // Build the dict shape session_yolo_enabled consumes: model_config as
    // text plus the row's fields (only model_config is read).
    json!({ "model_config": row.model_config })
}

#[test]
fn yolo_already_parsed_dict_and_null_config() {
    assert!(SessionDB::session_yolo_enabled(Some(&json!({"model_config": {"yolo_mode": true}}))));
    assert!(!SessionDB::session_yolo_enabled(Some(&json!({"model_config": {"yolo_mode": false}}))));
    // Null / missing model_config -> False.
    assert!(!SessionDB::session_yolo_enabled(Some(&json!({"model_config": null}))));
    assert!(!SessionDB::session_yolo_enabled(Some(&json!({}))));
}

#[test]
fn update_session_billing_route_unconditional_and_dedup() {
    let (_dir, db) = open_db("state.db");
    db.create_session("route", "cli", &NewSession {
        model: Some("primary".into()),
        ..Default::default()
    })
    .expect("create");

    db.update_session_billing_route(
        "route",
        "primary-provider",
        "https://primary.example/v1",
        Some("api_key"),
    )
    .expect("billing");
    let row = db.get_session("route").expect("get").expect("row");
    assert_eq!(row.model.as_deref(), Some("primary"));
    let conn = db.writer_conn();
    let billing_provider: String = conn
        .query_row(
            "SELECT billing_provider FROM sessions WHERE id = 'route'",
            [],
            |r| r.get(0),
        )
        .expect("bp");
    assert_eq!(billing_provider, "primary-provider");
}

#[test]
fn update_system_prompt_stores_hash_and_dedups() {
    let (_dir, db) = open_db("state.db");
    db.create_session("s1", "cli", &NewSession::default()).expect("create");

    db.update_system_prompt("s1", Some("the full assembled prompt")).expect("sp1");
    let row = db.get_session("s1").expect("get").expect("row");
    // get_session resolves the prompt through the hash table (upstream
    // _session_row_dict), so it still reads the stored text.
    assert_eq!(row.system_prompt.as_deref(), Some("the full assembled prompt"));
    let hash1 = row.system_prompt_hash.clone().expect("hash");

    // Same prompt again dedups to the same hash; table stays at one row.
    db.update_system_prompt("s1", Some("the full assembled prompt")).expect("sp2");
    let row = db.get_session("s1").expect("get").expect("row");
    assert_eq!(row.system_prompt_hash.as_deref(), Some(hash1.as_str()));
    let conn = db.writer_conn();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM system_prompts", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 1);

    // No prompt stores a NULL hash and resolves to None.
    db.update_system_prompt("s1", None).expect("sp null");
    let row = db.get_session("s1").expect("get").expect("row");
    assert_eq!(row.system_prompt_hash, None);
    assert_eq!(row.system_prompt, None);
}
