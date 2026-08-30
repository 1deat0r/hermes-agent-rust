//! Parity tests for `gateway/readiness.py` @ b9aa928, mirroring upstream
//! `tests/gateway/test_readiness.py` plus source-derived probe cases.
//! Env-var tests are serialized behind a mutex to avoid cross-test
//! `HERMES_HOME` races, per the workspace convention.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use serde_json::{json, Value};

use hermes_gateway::readiness::{collect_runtime_readiness, RuntimeReadinessInput};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_hermes_home<F: FnOnce(&Path)>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path().join(".hermes");
    fs::create_dir_all(&home).unwrap();
    // SAFETY: single-threaded with respect to env mutation (ENV_LOCK held).
    unsafe { std::env::set_var("HERMES_HOME", &home) };
    f(&home);
    unsafe { std::env::remove_var("HERMES_HOME") };
}

fn readiness(configured_model: &str, runtime_status: Option<Value>) -> Value {
    collect_runtime_readiness(RuntimeReadinessInput::new(
        configured_model,
        runtime_status.as_ref().and_then(Value::as_object),
    ))
}

fn check<'a>(result: &'a Value, name: &str) -> &'a Value {
    result["checks"].get(name).unwrap()
}

fn make_state_db(home: &Path) {
    let conn = rusqlite::Connection::open(home.join("state.db")).unwrap();
    conn.execute("CREATE TABLE probe (id INTEGER PRIMARY KEY)", [])
        .unwrap();
}

#[test]
fn reports_healthy_local_runtime() {
    with_hermes_home(|home| {
        fs::write(
            home.join("config.yaml"),
            "model:\n  provider: openrouter\n  model: test/model\n",
        )
        .unwrap();
        make_state_db(home);

        let runtime = json!({
            "gateway_state": "running",
            "platforms": {"telegram": {"state": "connected"}},
            "updated_at": "2026-07-09T00:00:00Z",
        });
        let mut input = RuntimeReadinessInput::new("test/model", runtime.as_object());
        input.active_api_runs = 2;
        let result = collect_runtime_readiness(input);

        assert_eq!(result["status"], "ok");
        assert_eq!(check(&result, "state_db")["status"], "ok");
        assert_eq!(check(&result, "config")["status"], "ok");
        assert_eq!(check(&result, "model")["status"], "ok");
        assert_eq!(check(&result, "gateway")["status"], "ok");
        assert_eq!(check(&result, "background_queues")["active_api_runs"], 2);
        let disk = check(&result, "disk")["status"].as_str().unwrap();
        assert!(["ok", "degraded"].contains(&disk));
    });
}

#[test]
fn degrades_on_invalid_config_and_stopped_gateway() {
    with_hermes_home(|home| {
        let config_path = home.join("config.yaml");
        fs::write(&config_path, "model: [unterminated").unwrap();

        let runtime = json!({"gateway_state": "stopped", "platforms": {}});
        let result = readiness("", Some(runtime));

        assert_eq!(result["status"], "degraded");
        assert_eq!(check(&result, "config")["status"], "degraded");
        assert_eq!(check(&result, "model")["status"], "degraded");
        assert_eq!(check(&result, "gateway")["status"], "degraded");
        // Readiness is diagnostic data, not an exception or a destructive
        // repair.
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model: [unterminated"
        );
    });
}

#[test]
fn uninitialized_home_degrades_but_stays_diagnostic() {
    with_hermes_home(|home| {
        // No config.yaml, no state.db: both probes report ok with the
        // "not initialized" / "using defaults" details.
        let result = readiness("test/model", None);
        assert_eq!(check(&result, "state_db")["status"], "ok");
        assert_eq!(check(&result, "state_db")["detail"], "not initialized");
        assert_eq!(check(&result, "config")["status"], "ok");
        assert_eq!(check(&result, "config")["detail"], "using defaults");
        assert_eq!(check(&result, "gateway")["state"], "unknown");
        assert_eq!(check(&result, "gateway")["status"], "degraded");
        assert!(home.join("state.db").exists() == false, "no db is created");
        assert!(!home.join("config.yaml").exists(), "no config is written");
    });
}

#[test]
fn corrupt_state_db_degrades_without_repair() {
    with_hermes_home(|home| {
        fs::write(home.join("state.db"), b"definitely not a sqlite database").unwrap();
        let result = readiness("test/model", None);
        assert_eq!(check(&result, "state_db")["status"], "degraded");
        // The corrupt file is left exactly as-is.
        assert_eq!(
            fs::read(home.join("state.db")).unwrap(),
            b"definitely not a sqlite database"
        );
    });
}

#[test]
fn config_top_level_non_mapping_degrades() {
    with_hermes_home(|home| {
        fs::write(home.join("config.yaml"), "- a\n- b\n").unwrap();
        let result = readiness("test/model", None);
        assert_eq!(check(&result, "config")["status"], "degraded");
        assert_eq!(
            check(&result, "config")["detail"],
            "top level is not a mapping"
        );
    });
}

#[test]
fn empty_config_document_counts_as_defaults() {
    with_hermes_home(|home| {
        // yaml.safe_load("") is None -> not a non-dict, so "ok".
        fs::write(home.join("config.yaml"), "").unwrap();
        let result = readiness("test/model", None);
        assert_eq!(check(&result, "config")["status"], "ok");
    });
}

#[test]
fn gateway_probe_counts_connected_platforms() {
    with_hermes_home(|_home| {
        let runtime = json!({
            "gateway_state": "draining",
            "platforms": {
                "telegram": {"state": "connected"},
                "whatsapp": {"status": "RUNNING"},
                "discord": {"state": "stopped"},
                "qqbot": "not-a-dict",
                "slack": {"state": "", "status": "ok"},
            },
        });
        let result = readiness("test/model", Some(runtime));
        let gateway = check(&result, "gateway");
        // draining is still "ok" per the accepted state set.
        assert_eq!(gateway["status"], "ok");
        assert_eq!(gateway["state"], "draining");
        assert_eq!(gateway["platforms"], 5);
        assert_eq!(gateway["connected_platforms"], 3);
    });
}

#[test]
fn model_probe_degrades_on_blank_names() {
    with_hermes_home(|_home| {
        assert_eq!(check(&readiness("", None), "model")["status"], "degraded");
        assert_eq!(
            check(&readiness("   ", None), "model")["status"],
            "degraded"
        );
        assert_eq!(
            check(&readiness("test/model", None), "model")["status"],
            "ok"
        );
    });
}

#[test]
fn queue_counters_clamp_negatives_and_overall_degrades() {
    with_hermes_home(|_home| {
        let mut input = RuntimeReadinessInput::new("test/model", None);
        input.active_api_runs = -3;
        input.process_completion_queue_depth = -1;
        input.active_delegations = 2;
        let result = collect_runtime_readiness(input);
        let queues = check(&result, "background_queues");
        // max(0, int(...)) per upstream.
        assert_eq!(queues["active_api_runs"], 0);
        assert_eq!(queues["process_completions"], 0);
        assert_eq!(queues["active_delegations"], 2);
        assert_eq!(queues["status"], "ok");
        // Missing home pieces (no state.db) degrade the overall despite the
        // ok queues: overall is ok only when every check is ok.
        assert_eq!(result["status"], "degraded");
    });
}
