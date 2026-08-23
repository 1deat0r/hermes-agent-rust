//! Configurable tool-output truncation limits.
//!
//! PARITY: tools/tool_output_limits.py @ b9aa928 (110 LOC, ported 1:1).
//!
//! Port-tracking: anomalyco/opencode PR #23770 (`feat(truncate): allow
//! configuring tool output truncation limits`).
//!
//! Hermes hardcoded the truncation thresholds in two places:
//! * `tools/terminal_tool.py` — `MAX_OUTPUT_CHARS = 50000` (terminal cap)
//! * `tools/file_operations.py` — `MAX_LINES = 2000` / `MAX_LINE_LENGTH = 2000`
//!
//! This module centralises those values behind a single config section
//! (`tool_output` in `config.yaml`) so power users can tune them without
//! patching the source. The existing hardcoded numbers remain as defaults, so
//! behaviour is unchanged when the config key is absent. The reader is
//! defensive: any error falls back to the built-in defaults so tools never
//! fail because of a malformed config.
//!
//! CONFIG SEAM (deferred): upstream reads `from hermes_cli.config import
//! load_config`. The hermes-cli config crate is not ported yet, so the Rust
//! module exposes an injectable loader hook (`set_config_loader`) that the
//! config crate wires when it lands; until then the defaults are returned.
//! This mirrors the upstream call site and keeps `coordinator`-side decisions
//! (how to find/parse config.yaml) out of this module. The loader's observable
//! contract matches the slice of `load_config()` upstream consumes here: an
//! optional JSON mapping whose `tool_output` section is read if it is an
//! object, ignored otherwise.
//!
//! DIVERGENCE NOTE (documented, invisible to upstream tests): for YAML floats
//! `.inf` / `.nan` upstream's `int(value)` raises `OverflowError`/`ValueError`
//! — `.inf` actually escapes the module's "never raises" contract because
//! `_coerce_positive_int` only catches `TypeError`/`ValueError`. The Rust
//! port's JSON seam cannot represent non-finite floats, and the coercion
//! treats them as invalid → default (strictly safer; upstream's crash is a bug
//! no caller relies on).

use std::sync::Mutex;

use serde_json::Value;

/// Hardcoded defaults — these match the pre-existing values, so adding this
/// module is behaviour-preserving for users who don't set `tool_output`.
///
/// PARITY: `DEFAULT_MAX_BYTES` (terminal_tool.MAX_OUTPUT_CHARS).
pub const DEFAULT_MAX_BYTES: usize = 50_000;
/// PARITY: `DEFAULT_MAX_LINES` (file_operations.MAX_LINES).
pub const DEFAULT_MAX_LINES: usize = 2000;
/// PARITY: `DEFAULT_MAX_LINE_LENGTH` (file_operations.MAX_LINE_LENGTH).
pub const DEFAULT_MAX_LINE_LENGTH: usize = 2000;

/// Resolved tool-output limits.
///
/// PARITY: the dict returned by `get_tool_output_limits()` — keys
/// `max_bytes`, `max_lines`, `max_line_length`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolOutputLimits {
    pub max_bytes: usize,
    pub max_lines: usize,
    pub max_line_length: usize,
}

impl Default for ToolOutputLimits {
    fn default() -> Self {
        ToolOutputLimits {
            max_bytes: DEFAULT_MAX_BYTES,
            max_lines: DEFAULT_MAX_LINES,
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
        }
    }
}

/// Config reader hook, wired by the hermes-config crate when it lands.
///
/// Mirrors the upstream `from hermes_cli.config import load_config` call
/// site: returns an optional full-config JSON mapping. `None` represents
/// "no config available" (missing file, parse error, reader raising — all
/// upstream paths that land on defaults).
pub type ConfigLoader = fn() -> Option<Value>;

static CONFIG_LOADER: Mutex<Option<ConfigLoader>> = Mutex::new(None);

fn with_loader<R>(f: impl FnOnce(Option<ConfigLoader>) -> R) -> R {
    match CONFIG_LOADER.lock() {
        Ok(guard) => f(*guard),
        Err(poisoned) => f(*poisoned.into_inner()),
    }
}

/// Wire the config reader (owned by hermes-config when that crate lands).
/// A loader set here shadows the built-in no-config behaviour.
pub fn set_config_loader(loader: ConfigLoader) {
    *CONFIG_LOADER.lock().unwrap_or_else(|e| e.into_inner()) = Some(loader);
}

/// Clear the injected config reader (restores built-in defaults).
pub fn clear_config_loader() {
    *CONFIG_LOADER.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Return `value` as a positive int, or `default` on any issue.
///
/// PARITY: `_coerce_positive_int` — `int(value)` semantics over the JSON
/// values a YAML config can yield: integers pass through, floats truncate
/// toward zero (Python `int(1.9) == 1`), bools become 0/1, numeric strings
/// parse (Python `int()` also strips whitespace), non-finite floats and all
/// other shapes fall back to `default`. Any result ≤ 0 falls back to default.
fn coerce_positive_int(value: Option<&Value>, default: usize) -> usize {
    let Some(value) = value else {
        return default; // None + _coerce_positive_int(None, …) -> default
    };
    let iv: i128 = match value {
        Value::Null => return default,
        Value::Bool(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i as i128
            } else if let Some(u) = n.as_u64() {
                u as i128
            } else if let Some(f) = n.as_f64() {
                // Upstream int(.inf) raises OverflowError; int(.nan) raises
                // ValueError -> default. JSON can't hold non-finite floats, but
                // guard anyway for any future non-JSON loader.
                if !f.is_finite() {
                    return default;
                }
                (f.trunc() as i64) as i128
            } else {
                return default;
            }
        }
        Value::String(s) => match s.trim().parse::<i64>() {
            Ok(i) => i as i128,
            Err(_) => return default,
        },
        Value::Array(_) | Value::Object(_) => return default,
    };
    if iv <= 0 {
        default
    } else {
        // Python's arbitrary-precision int can exceed u64 (e.g. a config value
        // of 2^70); clamp to usize::MAX on 64-bit. Behaviorally "effectively
        // unbounded" in both languages; exact value divergence is documented.
        u64::try_from(iv).unwrap_or(u64::MAX) as usize
    }
}

/// Resolve the `tool_output` config section into defaults-masked limits.
///
/// Mirrors upstream's `cfg.get("tool_output")` handling: non-dict config,
/// a non-dict section, or a missing section all land on `{}`.
fn resolve_limits() -> ToolOutputLimits {
    // Equivalent of upstream `cfg = load_config() or {}`: a missing (or
    // raising) config reader and a falsy config both land on {}.
    let cfg = match with_loader(|l| l) {
        Some(loader) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(loader))
            .ok()
            .flatten()
            .unwrap_or(Value::Null),
        None => Value::Null,
    };
    // `section = cfg.get("tool_output") if isinstance(cfg, dict) else None`.
    let section = match cfg {
        Value::Object(map) => map.get("tool_output").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    };
    let section = match section {
        Value::Object(map) => map,
        _ => Default::default(),
    };
    ToolOutputLimits {
        max_bytes: coerce_positive_int(section.get("max_bytes"), DEFAULT_MAX_BYTES),
        max_lines: coerce_positive_int(section.get("max_lines"), DEFAULT_MAX_LINES),
        max_line_length: coerce_positive_int(
            section.get("max_line_length"),
            DEFAULT_MAX_LINE_LENGTH,
        ),
    }
}

/// Module-level cache — populated on first call. Avoids repeated config file
/// I/O on every tool call.
///
/// PARITY: `_cached_limits` (process-lifetime cache dict; the Mutex replaces
/// Python's GIL serialization of the module global). A `Mutex<Option<_>>`
/// (rather than `OnceLock`) so tests can reset it exactly like upstream's
/// `_reset_tool_output_limits_cache`.
static CACHED_LIMITS: Mutex<Option<ToolOutputLimits>> = Mutex::new(None);

/// Return resolved tool-output limits, reading `tool_output` from the config
/// section. Missing or invalid entries fall through to the `DEFAULT_*`
/// constants. This function NEVER raises.
///
/// Result is cached for the process lifetime. Call
/// `reset_tool_output_limits_cache()` in tests that need a fresh read after
/// config changes.
pub fn get_tool_output_limits() -> ToolOutputLimits {
    let mut guard = CACHED_LIMITS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(limits) = *guard {
        return limits;
    }
    let limits = resolve_limits();
    *guard = Some(limits);
    limits
}

/// Reset the cached limits — for tests or after config hot-reload.
///
/// PARITY: `_reset_tool_output_limits_cache`.
pub fn reset_tool_output_limits_cache() {
    *CACHED_LIMITS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Shortcut for terminal-tool callers that only need the byte cap.
///
/// PARITY: `get_max_bytes`.
pub fn get_max_bytes() -> usize {
    get_tool_output_limits().max_bytes
}

/// Shortcut for file-ops callers that only need the line cap.
///
/// PARITY: `get_max_lines`.
pub fn get_max_lines() -> usize {
    get_tool_output_limits().max_lines
}

/// Shortcut for file-ops callers that only need the per-line cap.
///
/// PARITY: `get_max_line_length`.
pub fn get_max_line_length() -> usize {
    get_tool_output_limits().max_line_length
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static TEST_LOADER: std::sync::Mutex<
        Option<&'static (dyn Fn() -> Option<Value> + Send + Sync)>,
    > = std::sync::Mutex::new(None);

    fn test_loader_thunk() -> Option<Value> {
        TEST_LOADER.lock().unwrap().and_then(|f| f())
    }

    fn with_clean_cache<F: FnOnce()>(
        loader: impl Fn() -> Option<Value> + Send + Sync + 'static,
        f: F,
    ) {
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

    #[test]
    fn defaults_match_previous_hardcoded_values() {
        let _guard = TEST_SERIAL.lock().unwrap();
        assert_eq!(DEFAULT_MAX_BYTES, 50_000);
        assert_eq!(DEFAULT_MAX_LINES, 2000);
        assert_eq!(DEFAULT_MAX_LINE_LENGTH, 2000);
    }

    #[test]
    fn no_loader_returns_defaults() {
        let _guard = TEST_SERIAL.lock().unwrap();
        reset_tool_output_limits_cache();
        clear_config_loader();
        let limits = get_tool_output_limits();
        assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
        assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
        assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
        reset_tool_output_limits_cache();
    }

    #[test]
    fn user_config_overrides_all_three() {
        with_clean_cache(
            || {
                Some(json!({
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
            || Some(json!({"tool_output": "nonsense"})),
            || {
                let limits = get_tool_output_limits();
                assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
            },
        );
    }

    #[test]
    fn config_not_a_dict_falls_back() {
        with_clean_cache(
            || Some(json!("nonsense")),
            || {
                let limits = get_tool_output_limits();
                assert_eq!(limits.max_bytes, DEFAULT_MAX_BYTES);
                assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
                assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
            },
        );
    }

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        for bad in [
            Value::Null,
            json!("not a number"),
            json!(-1),
            json!(0),
            json!([]),
            json!({}),
        ] {
            with_clean_cache(
                move || {
                    Some(
                        json!({"tool_output": {"max_bytes": bad.clone(), "max_lines": bad.clone(), "max_line_length": bad.clone()}}),
                    )
                },
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
            || Some(json!({"tool_output": {"max_bytes": "75000"}})),
            || {
                let limits = get_tool_output_limits();
                assert_eq!(limits.max_bytes, 75_000);
                assert_eq!(limits.max_lines, DEFAULT_MAX_LINES);
                assert_eq!(limits.max_line_length, DEFAULT_MAX_LINE_LENGTH);
            },
        );
    }

    #[test]
    fn individual_accessors_delegate_to_get_tool_output_limits() {
        with_clean_cache(
            || {
                Some(json!({
                    "tool_output": {"max_bytes": 111, "max_lines": 222, "max_line_length": 333}
                }))
            },
            || {
                assert_eq!(get_max_bytes(), 111);
                assert_eq!(get_max_lines(), 222);
                assert_eq!(get_max_line_length(), 333);
            },
        );
    }

    #[test]
    fn false_and_true_coerce_like_python_int() {
        with_clean_cache(
            || Some(json!({"tool_output": {"max_bytes": false}})),
            || {
                assert_eq!(get_tool_output_limits().max_bytes, DEFAULT_MAX_BYTES);
                // int(False)=0 -> default
            },
        );
        with_clean_cache(
            || Some(json!({"tool_output": {"max_bytes": true}})),
            || {
                assert_eq!(get_tool_output_limits().max_bytes, 1); // int(True)=1
            },
        );
    }

    #[test]
    fn float_truncates_toward_zero() {
        with_clean_cache(
            || Some(json!({"tool_output": {"max_bytes": 1.9, "max_lines": 99.7}})),
            || {
                let limits = get_tool_output_limits();
                assert_eq!(limits.max_bytes, 1); // int(1.9) == 1
                assert_eq!(limits.max_lines, 99); // int(99.7) == 99
            },
        );
    }
}
