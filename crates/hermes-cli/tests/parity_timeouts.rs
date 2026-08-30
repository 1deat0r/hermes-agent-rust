// Tier: unit/mock — mirrors the precedence `tests/hermes_cli/test_timeouts.py`
// asserts through `AIAgent._resolved_api_call_timeout` (that consumer and the
// optional Anthropic SDK arm are unported, so the same priority chain is
// asserted directly against the module here).

use hermes_cli::timeouts::{get_provider_request_timeout_at, get_provider_stale_timeout_at};
use serde_json::{Map, Value};
use std::fs;
use tempfile::tempdir;

fn cfg(body: &str) -> Map<String, Value> {
    let yaml = hermes_utils::fast_safe_load(body).expect("test yaml parses");
    serde_json::to_value(yaml)
        .expect("yaml to json")
        .as_object()
        .expect("object config")
        .clone()
}

const OPENROUTER: &str = r#"
providers:
  openrouter:
    request_timeout_seconds: 77
    stale_timeout_seconds: 120
    models:
      openai/gpt-4o-mini:
        timeout_seconds: 42
        stale_timeout_seconds: 21
      broken/model: not-a-mapping
"#;

#[test]
fn per_model_override_wins_over_the_provider_knob() {
    let config = cfg(OPENROUTER);
    assert_eq!(
        get_provider_request_timeout_at(&config, "openrouter", Some("openai/gpt-4o-mini")),
        Some(42.0)
    );
    assert_eq!(
        get_provider_stale_timeout_at(&config, "openrouter", Some("openai/gpt-4o-mini")),
        Some(21.0)
    );
}

#[test]
fn provider_knob_applies_when_the_model_has_no_override() {
    let config = cfg(OPENROUTER);
    assert_eq!(
        get_provider_request_timeout_at(&config, "openrouter", Some("some/other-model")),
        Some(77.0)
    );
    assert_eq!(
        get_provider_request_timeout_at(&config, "openrouter", None),
        Some(77.0)
    );
    assert_eq!(
        get_provider_stale_timeout_at(&config, "openrouter", Some("unknown/model")),
        Some(120.0)
    );
}

#[test]
fn non_mapping_model_config_falls_back_to_the_provider_knob() {
    let config = cfg(OPENROUTER);
    // `models.broken/model` is a string, so `_get_model_config` yields no dict
    // and the provider-wide value applies.
    assert_eq!(
        get_provider_request_timeout_at(&config, "openrouter", Some("broken/model")),
        Some(77.0)
    );
}

#[test]
fn unknown_provider_or_blank_id_yield_none() {
    let config = cfg(OPENROUTER);
    assert_eq!(get_provider_request_timeout_at(&config, "nous", None), None);
    assert_eq!(get_provider_request_timeout_at(&config, "", None), None);
    assert_eq!(
        get_provider_stale_timeout_at(&config, "openrouter", Some("openai/gpt-4o-mini")),
        Some(21.0)
    );
    // A provider entry that is not a mapping is ignored.
    let scalar = cfg("providers:\n  openrouter: 5\n");
    assert_eq!(
        get_provider_request_timeout_at(&scalar, "openrouter", None),
        None
    );
}

#[test]
fn coercion_rejects_non_positive_and_unparseable_values() {
    let config = cfg(r#"
providers:
  zero:
    request_timeout_seconds: 0
  negative:
    request_timeout_seconds: -5
  text:
    request_timeout_seconds: "abc"
  numeric-text:
    request_timeout_seconds: "45.5"
  boolean:
    request_timeout_seconds: true
  missing: {}
"#);
    assert_eq!(get_provider_request_timeout_at(&config, "zero", None), None);
    assert_eq!(
        get_provider_request_timeout_at(&config, "negative", None),
        None
    );
    assert_eq!(get_provider_request_timeout_at(&config, "text", None), None);
    assert_eq!(
        get_provider_request_timeout_at(&config, "numeric-text", None),
        Some(45.5)
    );
    // `float(True) == 1.0` in Python, so a boolean is a valid 1s timeout.
    assert_eq!(
        get_provider_request_timeout_at(&config, "boolean", None),
        Some(1.0)
    );
    assert_eq!(
        get_provider_request_timeout_at(&config, "missing", None),
        None
    );
}

#[test]
fn malformed_or_absent_config_file_fails_open_to_none() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    fs::write(&path, "providers: [unclosed\n").expect("write");
    assert_eq!(
        get_provider_request_timeout_at_path(&path, "openrouter", None),
        None
    );

    let missing = td.path().join("nope.yaml");
    assert_eq!(
        get_provider_request_timeout_at_path(&missing, "openrouter", None),
        None
    );
}

// Re-export shim so the last test reads like the others without pulling the
// path-taking helper into every assertion.
use hermes_cli::timeouts::get_provider_request_timeout_at_path;
