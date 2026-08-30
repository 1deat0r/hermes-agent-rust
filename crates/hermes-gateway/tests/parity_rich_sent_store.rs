//! Parity tests for `gateway/rich_sent_store.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); these cases derive from the upstream code as oracle. Env-var
//! tests are serialized behind a mutex to avoid cross-test `HERMES_HOME`
//! races, per the workspace convention.

use std::fs;
use std::sync::Mutex;

use serde_json::Value;

// One lock per binary; every test in this file takes it.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_hermes_home<F: FnOnce(&std::path::Path)>(f: F) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    // SAFETY: single-threaded with respect to env mutation (ENV_LOCK held).
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };
    f(td.path());
    unsafe { std::env::remove_var("HERMES_HOME") };
}

fn index_at(home: &std::path::Path) -> Value {
    let raw = fs::read_to_string(home.join("state/rich_sent_index.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn key_format_is_chat_colon_message() {
    with_hermes_home(|home| {
        hermes_gateway::rich_sent_store::record(Some(42), Some(7), Some("hello"));
        let data = index_at(home);
        assert!(data.get("42:7").is_some(), "unexpected keys: {data}");
        assert_eq!(data.get("42:7").unwrap()["t"], "hello");
    });
}

#[test]
fn lookup_round_trips_recorded_text() {
    with_hermes_home(|_| {
        hermes_gateway::rich_sent_store::record(Some(1), Some(2), Some("launch brief"));
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)).as_deref(),
            Some("launch brief")
        );
    });
}

#[test]
fn record_truncates_text_to_max_chars() {
    with_hermes_home(|home| {
        let long = "x".repeat(2500);
        hermes_gateway::rich_sent_store::record(Some(1), Some(2), Some(&long));
        assert_eq!(
            index_at(home)["1:2"]["t"].as_str().unwrap().chars().count(),
            2000
        );
    });
}

#[test]
fn record_guards_mirror_python_falsiness() {
    with_hermes_home(|home| {
        // Empty text, None text, None ids: all return before any disk touch.
        hermes_gateway::rich_sent_store::record(Some(1), Some(2), Some(""));
        hermes_gateway::rich_sent_store::record(Some(1), Some(2), None);
        hermes_gateway::rich_sent_store::record(None, Some(2), Some("t"));
        hermes_gateway::rich_sent_store::record(Some(1), None, Some("t"));
        assert!(!home.join("state").exists(), "no store file may be created");
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)),
            None
        );
    });
}

#[test]
fn lookup_fail_open_on_missing_or_corrupt_or_non_dict_store() {
    with_hermes_home(|home| {
        // Missing file.
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)),
            None
        );
        fs::create_dir_all(home.join("state")).unwrap();
        // Corrupt JSON (ValueError).
        fs::write(home.join("state/rich_sent_index.json"), "{not json").unwrap();
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)),
            None
        );
        // Non-dict document (AttributeError arm).
        fs::write(home.join("state/rich_sent_index.json"), "[1, 2]").unwrap();
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)),
            None
        );
    });
}

#[test]
fn lookup_empty_or_null_text_is_none() {
    with_hermes_home(|home| {
        fs::create_dir_all(home.join("state")).unwrap();
        fs::write(
            home.join("state/rich_sent_index.json"),
            r#"{"1:2": {"t": "", "ts": 1}, "3:4": {"t": null, "ts": 1}}"#,
        )
        .unwrap();
        // Python `entry.get("t") or None`.
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(1), Some(2)),
            None
        );
        assert_eq!(
            hermes_gateway::rich_sent_store::lookup(Some(3), Some(4)),
            None
        );
    });
}

#[test]
fn trim_oldest_by_timestamp_past_cap() {
    with_hermes_home(|home| {
        // All records land inside the same wall-clock second, so their `ts`
        // values tie and Python's stable sort keeps insertion order — the
        // first-inserted keys are the ones trimmed.
        for message_id in 0..1002 {
            hermes_gateway::rich_sent_store::record(Some(1), Some(message_id), Some("t"));
        }
        let data = index_at(home);
        assert_eq!(data.as_object().unwrap().len(), 1000);
        assert!(
            data.get("1:0").is_none(),
            "oldest inserted key must be trimmed"
        );
        assert!(data.get("1:1").is_none());
        assert!(data.get("1:2").is_some());
        assert!(data.get("1:1001").is_some(), "newest key must survive");
    });
}

#[test]
fn record_recovers_from_corrupt_store() {
    with_hermes_home(|home| {
        fs::create_dir_all(home.join("state")).unwrap();
        fs::write(home.join("state/rich_sent_index.json"), "corrupt").unwrap();
        hermes_gateway::rich_sent_store::record(Some(5), Some(6), Some("fresh"));
        // FileNotFoundError/ValueError -> {} then the record lands.
        let data = index_at(home);
        assert_eq!(data["5:6"]["t"], "fresh");
        assert_eq!(data.as_object().unwrap().len(), 1);
    });
}
