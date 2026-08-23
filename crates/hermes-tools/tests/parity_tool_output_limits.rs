//! Parity oracles for tools/tool_output_limits.rs.
//!
//! Mirrors upstream tests/tools/test_tool_output_limits.py @ b9aa928.
//! Port-tracking: anomalyco/opencode PR #23770
//! (`feat(truncate): allow configuring tool output truncation limits`).
//!
//! Upstream patches `hermes_cli.config.load_config` with mock.patch; the Rust
//! port exposes the same seam as an injectable `set_config_loader` hook (see
//! module doc). Each test resets the process-lifetime cache first, exactly
//! like the autouse `_reset_limits_cache` fixture.
//!
//! DEFERRED (unported subsystem): `TestDefaultConfigHasSection` reads
//! `hermes_cli.config.DEFAULT_CONFIG` — the hermes-cli config crate is not
//! ported yet; DEFAULT_CONFIG's `tool_output` section equals the DEFAULT_*
//! constants here by construction upstream, so the assertion becomes
//! tautological once the config crate lands. `TestIntegrationReadPagination`
//! exercises `file_operations.normalize_read_pagination` — unported.

use hermes_tools::tool_output_limits::{
    clear_config_loader, get_max_bytes, get_max_line_length, get_max_lines, get_tool_output_limits,
    reset_tool_output_limits_cache, set_config_loader, DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES,
    DEFAULT_MAX_LINE_LENGTH,
};
use serde_json::Value;

static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_clean_cache<F: FnOnce()>(loader: impl Fn() -> Option<Value> + Send + Sync + 'static, f: F) {
    let _guard = TEST_SERIAL.lock().unwrap();
    reset_tool_output_limits_cache();
    clear_config_loader();
    // The production slot is a fn pointer (no captures): route through a
    // static slot so the test closure is reachable without captures.
    *TEST_LOADER.lock().unwrap() = Some(Box::leak(Box::new(loader)));
    set_config_loader(test_loader_thunk);
    f();
    reset_tool_output_limits_cache();
    clear_config_loader();
    *TEST_LOADER.lock().unwrap() = None;
}

static TEST_LOADER: std::sync::Mutex<Option<&'static (dyn Fn() -> Option<Value> + Send + Sync)>> =
    std::sync::Mutex::new(None);

fn test_loader_thunk() -> Option<Value> {
    TEST_LOADER.lock().unwrap().and_then(|f| f())
}

fn all_invalid(bad: Value) -> Value {
    serde_json::json!({ "tool_output": { "max_bytes": bad, "max_lines": bad, "max_line_length": bad } })
}

// ── Defaults ────────────────────────────────────────────────────────────

#[test]
fn defaults_match_previous_hardcoded_values() {
    let _guard = TEST_SERIAL.lock().unwrap();
    assert_eq!(DEFAULT_MAX_BYTES, 50_000);
    assert_eq!(DEFAULT_MAX_LINES, 2000);
    assert_eq!(DEFAULT_MAX_LINE_LENGTH, 2000);
}

#[test]
fn get_limits_returns_defaults_when_load_config_raises() {
    // Upstream patches load_config with a raising side_effect; the Rust seam
    // folds "config reader unavailable" into a None return — both land on the
    // same defaults path (module never raises).
    with_clean_cache(
        || None,
        || {
            let limits = get_tool_output_limits();
            assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
        },
    );
}

#[test]
fn get_limits_returns_defaults_with_no_loader_wired() {
    let _guard = TEST_SERIAL.lock().unwrap();
    // The config crate is not ported yet — no loader means built-in defaults.
    reset_tool_output_limits_cache();
    clear_config_loader();
    let limits = get_tool_output_limits();
    assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
    assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
    reset_tool_output_limits_cache();
}

// ── Overrides ───────────────────────────────────────────────────────────

#[test]
fn user_config_overrides_all_three() {
    with_clean_cache(
        || {
            Some(serde_json::json!({
                "tool_output": {
                    "max_bytes": 100_000,
                    "max_lines": 5000,
                    "max_line_length": 4096,
                }
            }))
        },
        || {
            let limits = get_tool_output_limits();
            assert_eq!(limits.max_bytes, 100_000);
            assert_eq!(limits.max_lines, 5000);
            assert_eq!(limits.max_line_length, 4096);
        },
    );
}

#[test]
fn section_not_a_dict_falls_back() {
    with_clean_cache(
        || Some(serde_json::json!({ "tool_output": "nonsense" })),
        || {
            let limits = get_tool_output_limits();
            assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
        },
    );
}

#[test]
fn config_not_a_dict_falls_back() {
    // Upstream: `cfg.get("tool_output") if isinstance(cfg, dict) else None`.
    with_clean_cache(
        || Some(serde_json::json!("not a dict")),
        || {
            let limits = get_tool_output_limits();
            assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
            assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
            assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
        },
    );
}

// ── Coercion ────────────────────────────────────────────────────────────

#[test]
fn invalid_values_fall_back_to_defaults() {
    for bad in [
        Value::Null,
        serde_json::json!("not a number"),
        serde_json::json!(-1),
        serde_json::json!(0),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        with_clean_cache(
            move || Some(all_invalid(bad.clone())),
            || {
                let limits = get_tool_output_limits();
                assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
                assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
                assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
            },
        );
    }
}

#[test]
fn string_integer_is_coerced() {
    with_clean_cache(
        || Some(serde_json::json!({ "tool_output": { "max_bytes": "75000" } })),
        || {
            let limits = get_tool_output_limits();
            assert_eq!(limits.max_bytes, 75_000);
        },
    );
}

// ── Shortcuts ───────────────────────────────────────────────────────────

#[test]
fn individual_accessors_delegate_to_get_tool_output_limits() {
    with_clean_cache(
        || {
            Some(serde_json::json!({
                "tool_output": {
                    "max_bytes": 111,
                    "max_lines": 222,
                    "max_line_length": 333,
                }
            }))
        },
        || {
            assert_eq!(get_max_bytes(), 111);
            assert_eq!(get_max_lines(), 222);
            assert_eq!(get_max_line_length(), 333);
        },
    );
}
