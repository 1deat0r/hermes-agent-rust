//! LM Studio reasoning-effort resolution.
//!
//! PARITY: `agent/lmstudio_reasoning.py` @ b9aa928 (whole module, lines 1-61).
//!
//! LM Studio publishes per-model `capabilities.reasoning.allowed_options` (for
//! example `["off","on"]` for toggle-style models or `["off","minimal","low"]`
//! for graduated ones). The user's `reasoning_config` is mapped onto LM
//! Studio's OpenAI-compatible vocabulary and then clamped against the model's
//! allowed set so the server does not 400 on an unsupported effort.

use serde_json::{Map, Value};

/// PARITY: `_LM_VALID_EFFORTS` (upstream line 14).
const LM_VALID_EFFORTS: [&str; 6] = ["none", "minimal", "low", "medium", "high", "xhigh"];

/// PARITY: `_LM_EFFORT_ALIASES` (upstream line 17). Toggle-style models publish
/// `allowed_options` as `["off","on"]` in `/api/v1/models`; this maps them onto
/// the OpenAI-compatible request vocabulary. Applied to BOTH sides, unlike the
/// clamp below.
const LM_EFFORT_ALIASES: [(&str, &str); 2] = [("off", "none"), ("on", "medium")];

/// PARITY: `_LM_EFFORT_CLAMP` (upstream lines 19-29). Hermes' generic ladder
/// grew past LM Studio's vocabulary; the stronger generic levels clamp onto LM
/// Studio's ceiling instead of missing `_LM_VALID_EFFORTS` and silently keeping
/// the `"medium"` default. Deliberately separate from the alias table because
/// the aliases are also applied to the model's published `allowed_options`,
/// which must not be rewritten.
const LM_EFFORT_CLAMP: [(&str, &str); 2] = [("max", "xhigh"), ("ultra", "xhigh")];

fn alias(effort: &str) -> &str {
    LM_EFFORT_ALIASES
        .iter()
        .find(|(from, _)| *from == effort)
        .map_or(effort, |(_, to)| to)
}

fn clamp(effort: &str) -> &str {
    LM_EFFORT_CLAMP
        .iter()
        .find(|(from, _)| *from == effort)
        .map_or(effort, |(_, to)| to)
}

/// Return the `reasoning_effort` string to send to LM Studio, or `None`.
///
/// PARITY: `resolve_lmstudio_effort` (upstream lines 32-61). `None` means
/// "omit the field": the user picked a level the model cannot honor, so let LM
/// Studio fall back to the model's declared default rather than silently
/// substituting a different effort. When `allowed_options` is falsy (the probe
/// failed), clamping is skipped and the resolved effort is sent anyway.
///
/// The `reasoning_config` argument reproduces the source's `if reasoning_config
/// and isinstance(...)` guard, so an empty map takes the `"medium"` default
/// exactly like an empty dict does in Python.
///
/// Divergence (fail-open where Python raises): a non-string, non-falsy
/// `effort` such as `5` reaches `.strip()` upstream and raises
/// `AttributeError`; the same shape in a published `allowed_options` entry
/// raises there through `dict.get(unhashable)`. Both take the neutral branch
/// here — the empty-string effort path and the verbatim option path — because
/// Rust has no exception channel to propagate and the observable result for a
/// well-formed config is identical.
pub fn resolve_lmstudio_effort(
    reasoning_config: Option<&Map<String, Value>>,
    allowed_options: Option<&[&str]>,
) -> Option<String> {
    let mut effort = "medium".to_string();
    if let Some(config) = reasoning_config.filter(|config| !config.is_empty()) {
        // `reasoning_config.get("enabled") is False` — identity against False,
        // so an explicit null takes the effort branch instead.
        if matches!(config.get("enabled"), Some(Value::Bool(false))) {
            effort = "none".to_string();
        } else {
            let raw = match config.get("effort") {
                Some(Value::String(text)) if !text.is_empty() => text.clone(),
                _ => String::new(),
            };
            let normalized = raw.trim().to_lowercase();
            let raw = clamp(alias(&normalized));
            if LM_VALID_EFFORTS.contains(&raw) {
                effort = raw.to_string();
            }
        }
    }
    if let Some(allowed_options) = allowed_options.filter(|options| !options.is_empty()) {
        let allowed: Vec<&str> = allowed_options.iter().copied().map(alias).collect();
        if !allowed.contains(&effort.as_str()) {
            return None;
        }
    }
    Some(effort)
}
