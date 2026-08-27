// Tier: unit — mirrors tests/agent/test_verify_hooks.py.

use hermes_agent::verify_hooks::{
    coding_verify_guidance, max_verify_nudges, CODING_VERIFY_GUIDANCE, DEFAULT_MAX_VERIFY_NUDGES,
};
use serde_json::{json, Map, Value};
use std::ffi::OsString;
use tempfile::tempdir;

static VERIFY_HOME_MUTEX: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// PARITY: `_agent_cfg(None)` falling back to `load_config()`.
struct HermesHome {
    previous: Option<OsString>,
    _td: tempfile::TempDir,
}

impl HermesHome {
    fn with_config(body: &str) -> Self {
        let td = tempdir().expect("tempdir");
        std::fs::write(td.path().join("config.yaml"), body).expect("write config");
        let previous = std::env::var_os("HERMES_HOME");
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        Self { previous, _td: td }
    }
}

impl Drop for HermesHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var("HERMES_HOME", value) },
            None => unsafe { std::env::remove_var("HERMES_HOME") },
        }
    }
}

fn cfg(value: Value) -> Map<String, Value> {
    value.as_object().expect("object config").clone()
}

#[test]
fn default_when_unset() {
    assert_eq!(
        max_verify_nudges(Some(&Map::new())),
        DEFAULT_MAX_VERIFY_NUDGES
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {}})))),
        DEFAULT_MAX_VERIFY_NUDGES
    );
}

#[test]
fn reads_and_coerces() {
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": 5}})))),
        5
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": "2"}})))),
        2
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": -1}})))),
        0,
        "the bound is clamped at zero"
    );
}

#[test]
fn bad_value_falls_back() {
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": "x"}})))),
        DEFAULT_MAX_VERIFY_NUDGES
    );
    // Python `int()` also rejects a float literal written as a string, and a
    // null/absent agent section is a TypeError path upstream.
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": "2.5"}})))),
        DEFAULT_MAX_VERIFY_NUDGES
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": null}})))),
        DEFAULT_MAX_VERIFY_NUDGES
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": "oops_a_string"})))),
        DEFAULT_MAX_VERIFY_NUDGES
    );
}

#[test]
fn float_and_bool_coercion_match_python_int() {
    // `int(5.9)` truncates toward zero; `int(True)` is 1.
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": 5.9}})))),
        5
    );
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": true}})))),
        1
    );
    // Whitespace is tolerated by `int(" 4 ")`.
    assert_eq!(
        max_verify_nudges(Some(&cfg(json!({"agent": {"max_verify_nudges": " 4 "}})))),
        4
    );
}

#[test]
fn guidance_enabled_by_default() {
    assert_eq!(
        coding_verify_guidance(Some(&Map::new())),
        Some(CODING_VERIFY_GUIDANCE)
    );
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {}})))),
        Some(CODING_VERIFY_GUIDANCE)
    );
}

#[test]
fn guidance_reads_truthy_config() {
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {"verify_guidance": "yes"}})))),
        Some(CODING_VERIFY_GUIDANCE)
    );
    // Explicit null keeps the `default=True` answer.
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {"verify_guidance": null}})))),
        Some(CODING_VERIFY_GUIDANCE)
    );
}

#[test]
fn guidance_opt_out_via_config() {
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {"verify_guidance": false}})))),
        None
    );
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {"verify_guidance": "off"}})))),
        None
    );
    assert_eq!(
        coding_verify_guidance(Some(&cfg(json!({"agent": {"verify_guidance": ""}})))),
        None
    );
}

#[test]
fn guidance_text_is_the_shipped_wording() {
    assert!(CODING_VERIFY_GUIDANCE.starts_with("[Coding] Before you run tests"));
    assert!(CODING_VERIFY_GUIDANCE.ends_with("concise, efficient, and elegant."));
    assert!(CODING_VERIFY_GUIDANCE.contains("hold off on tests and linters"));
    assert!(CODING_VERIFY_GUIDANCE.contains("KISS/DRY"));
}

#[test]
fn none_config_reads_the_process_default_path() {
    let _lock = VERIFY_HOME_MUTEX.lock();
    let _home =
        HermesHome::with_config("agent:\n  max_verify_nudges: \"7\"\n  verify_guidance: false\n");

    assert_eq!(max_verify_nudges(None), 7);
    assert_eq!(coding_verify_guidance(None), None);
}

#[test]
fn none_config_fails_open_when_no_file_exists() {
    let _lock = VERIFY_HOME_MUTEX.lock();
    let _home = HermesHome::with_config("");
    // The temp config is an empty document, so both helpers fall back.
    assert_eq!(max_verify_nudges(None), DEFAULT_MAX_VERIFY_NUDGES);
    assert_eq!(coding_verify_guidance(None), Some(CODING_VERIFY_GUIDANCE));
}
