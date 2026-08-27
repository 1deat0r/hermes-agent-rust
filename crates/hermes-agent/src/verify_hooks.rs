//! Verification-loop helpers for the `pre_verify` round-end gate.
//!
//! PARITY: `agent/verify_hooks.py` @ b9aa928 (whole module, lines 1-70).
//!
//! When the agent has edited code and is about to verify/finish, the loop fires
//! the `pre_verify` hook (user directives resolved by
//! `hermes_cli.plugins.get_pre_verify_continue_message`). A directive keeps the
//! agent going one more turn — run a check, defer it, tidy the diff — instead of
//! stopping immediately.
//!
//! The shipped guidance lives on the evidence-based verification-stop nudge
//! (`agent/verification_stop.py`), not as a second default stop gate. That keeps
//! the default token cost tied to the existing "missing verification evidence"
//! decision while preserving `pre_verify` for user/plugin policy.
//!
//! PENDING SEAM: the `pre_verify` user/plugin hook aggregation itself lives in
//! `hermes_cli.plugins` and is not ported; only the policy helpers are here.

use crate::config::{json_truthy, load_merged_config_snapshot};
use serde_json::{Map, Value};

/// PARITY: `DEFAULT_MAX_VERIFY_NUDGES` (upstream line 18).
pub const DEFAULT_MAX_VERIFY_NUDGES: i64 = 3;

/// PARITY: `CODING_VERIFY_GUIDANCE` (upstream lines 20-29), byte for byte.
pub const CODING_VERIFY_GUIDANCE: &str =
    "[Coding] Before you run tests/linters or call this done: if this is \
creative UI/visual work, hold off on tests and linters until the user says \
they like the result or you're about to commit. And before every commit, \
clean your work: keep it KISS/DRY, match the surrounding code style, and be \
elitist, shorthand, clever, concise, efficient, and elegant.";

/// PARITY: `is_truthy_value` (`utils.py` lines 22-30): `None` takes the
/// default, a bool is itself, a string is trimmed/lowercased against the
/// project truthy set, and anything else falls back to `bool(value)`.
fn is_truthy_value(value: Option<&Value>, default: bool) -> bool {
    match value {
        None | Some(Value::Null) => default,
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => {
            let lowered = value.trim().to_lowercase();
            matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
        }
        Some(other) => json_truthy(Some(other)),
    }
}

/// PARITY: `_agent_cfg` (upstream lines 52-61): `None` mirrors the source's
/// own `load_config()` read (whose failure path is an empty config); a
/// non-mapping `agent` value resolves to the empty config exactly like the
/// source's `isinstance` guard. The CLI default catalog is above this crate,
/// and its shipped `agent` defaults — `verify_guidance: True` and
/// `max_verify_nudges: 3` (`hermes_cli/config_defaults.py` lines 173-176) —
/// are identical to the fallbacks these two helpers apply when the section or
/// key is absent, so the effective answer cannot drift.
fn agent_cfg(config: Option<&Map<String, Value>>) -> Map<String, Value> {
    let resolved = match config {
        Some(config) => config.clone(),
        None => load_merged_config_snapshot(&Map::new()).pool_config,
    };
    match resolved.get("agent") {
        Some(Value::Object(agent)) => agent.clone(),
        _ => Map::new(),
    }
}

/// Bound on consecutive `pre_verify` continue directives per turn (`>= 0`).
///
/// PARITY: `max_verify_nudges` (upstream lines 32-39): `max(0, int(raw))` with
/// the default recovered from `TypeError`/`ValueError`, so a bool counts as
/// Python's `int(True) == 1`, a float truncates toward zero, and a
/// whitespace-wrapped integer string parses while `"2.5"` does not.
pub fn max_verify_nudges(config: Option<&Map<String, Value>>) -> i64 {
    let agent = agent_cfg(config);
    let coerced = match agent.get("max_verify_nudges") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(value)) => Some(i64::from(*value)),
        Some(Value::Number(number)) => number.as_i64().or_else(|| {
            number
                .as_f64()
                .filter(|float| float.is_finite())
                .map(|float| float.trunc() as i64)
        }),
        Some(Value::String(text)) => text.trim().parse::<i64>().ok(),
        Some(_) => None,
    };
    coerced.map_or(DEFAULT_MAX_VERIFY_NUDGES, |value| value.max(0))
}

/// Return the optional guidance appended to verification-stop nudges.
///
/// PARITY: `coding_verify_guidance` (upstream lines 42-46): the key is read
/// with a `True` default, so an absent setting keeps the guidance and an
/// explicit null keeps it through `is_truthy_value(..., default=True)`, while
/// `false` or any non-truthy spelling opts out.
pub fn coding_verify_guidance(config: Option<&Map<String, Value>>) -> Option<&'static str> {
    let agent = agent_cfg(config);
    let enabled = match agent.get("verify_guidance") {
        None => true,
        Some(value) => is_truthy_value(Some(value), true),
    };
    if enabled {
        Some(CODING_VERIFY_GUIDANCE)
    } else {
        None
    }
}
