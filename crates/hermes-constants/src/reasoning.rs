//! Reasoning-effort parsing and spelling-tolerant model variant generation.
//!
//! PARITY: hermes_constants.py lines 942–1097 (VALID_REASONING_EFFORTS,
//! parse_reasoning_effort, _canonical_model_variants,
//! resolve_per_model_reasoning_effort). NOTE: resolve_per_model_reasoning_effort
//! and resolve_reasoning_config are deferred to P1 (they consume a config
//! dict); `canonical_model_variants` is public here because it is pure and
//! fully tested.

use serde::{Deserialize, Serialize};

/// Valid effort levels (upstream tuple order matters for docs/tests).
pub const VALID_REASONING_EFFORTS: [&str; 7] =
    ["minimal", "low", "medium", "high", "xhigh", "max", "ultra"];

/// Parsed reasoning config dict — mirrors upstream return values:
/// `{"enabled": false}` or `{"enabled": true, "effort": <level>}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// Parse a reasoning effort level into a config dict.
///
/// Valid levels: "none", "minimal", "low", "medium", "high", "xhigh", "max",
/// "ultra". Returns `None` when the input is empty or unrecognized (caller
/// uses default). Returns `{enabled: false}` for "none" (aliases: "false",
/// "disabled", and boolean `false` — users write `reasoning_effort: false` /
/// `off` / `no` in config.yaml and YAML hands us a bool, which must mean
/// disabled, not "fall back to the default and keep thinking"). Returns
/// `{enabled: true, effort: <level>}` for valid effort levels.
///
/// PARITY: hermes_constants.py `parse_reasoning_effort` (947–972).
pub fn parse_reasoning_effort<'a>(effort: impl Into<EffortInput<'a>>) -> Option<ReasoningConfig> {
    let input = effort.into();
    match input {
        EffortInput::Bool(false) => Some(ReasoningConfig { enabled: false, effort: None }),
        EffortInput::Bool(true) | EffortInput::None => None,
        EffortInput::Str(s) => {
            if s.trim().is_empty() {
                return None;
            }
            let e = s.trim().to_lowercase();
            if matches!(e.as_str(), "none" | "false" | "disabled") {
                return Some(ReasoningConfig { enabled: false, effort: None });
            }
            if VALID_REASONING_EFFORTS.contains(&e.as_str()) {
                return Some(ReasoningConfig { enabled: true, effort: Some(e) });
            }
            None
        }
    }
}

/// Input coercion for [`parse_reasoning_effort`] mirroring Python's `str()`
/// on arbitrary values. For YAML scalars this covers the real surface (bool,
/// string, null). Non-string types upstream would stringify (`str(effort)`)
/// and then fail the valid set anyway.
#[derive(Debug, Clone)]
pub enum EffortInput<'a> {
    Bool(bool),
    None,
    Str(&'a str),
}

impl<'a> From<&'a str> for EffortInput<'a> {
    fn from(v: &'a str) -> Self {
        EffortInput::Str(v)
    }
}
impl From<bool> for EffortInput<'_> {
    fn from(v: bool) -> Self {
        EffortInput::Bool(v)
    }
}
impl<'a> From<Option<&'a str>> for EffortInput<'a> {
    fn from(v: Option<&'a str>) -> Self {
        match v {
            Some(s) => EffortInput::Str(s),
            None => EffortInput::None,
        }
    }
}

// ── Spelling-tolerant variant generation (upstream algorithm, step-for-step) ──

fn dash_to_dot(re: &regex::Regex, s: &str) -> String {
    re.replace_all(s, "$1.$2").into_owned()
}

fn dot_to_dash(re: &regex::Regex, s: &str) -> String {
    re.replace_all(s, "$1-$2").into_owned()
}

fn add(v: String, seen: &mut std::collections::HashSet<String>, variants: &mut Vec<String>) {
    if !v.is_empty() && !seen.contains(&v) {
        seen.insert(v.clone());
        variants.push(v);
    }
}

fn add_with_derivatives(
    s: &str,
    dash_dot_re: &regex::Regex,
    dot_dash_re: &regex::Regex,
    seen: &mut std::collections::HashSet<String>,
    variants: &mut Vec<String>,
) {
    add(s.to_string(), seen, variants);
    let all_dashed = s.replace('.', "-");
    add(all_dashed.clone(), seen, variants);
    let all_dotted = s.replace('-', ".");
    add(all_dotted.clone(), seen, variants);
    add(dash_to_dot(dash_dot_re, s), seen, variants);
    add(dot_to_dash(dot_dash_re, s), seen, variants);
    add(dash_to_dot(dash_dot_re, &all_dashed), seen, variants);
    add(dot_to_dash(dot_dash_re, &all_dotted), seen, variants);
}

/// Generate bounded spelling variants for tolerant override matching.
///
/// Ports upstream's algorithm step-for-step: exact input, dots/dashes
/// cross-substitution, version-dot recovery on all derivatives, provider
/// prefix stripping, prefix re-adding. Duplicates removed in insertion order
/// (exact always wins). Parity-pinned by golden fixture
/// `upstream/golden_constants_reasoning.json` (292 variants for
/// `claude-opus-4.5`).
///
/// PARITY: hermes_constants.py `_canonical_model_variants` (974–1062).
pub fn canonical_model_variants(model: &str) -> Vec<String> {
    let dash_to_dot_re = regex::Regex::new(r"(\d)-(\d)").unwrap();
    let dot_to_dash_re = regex::Regex::new(r"(\d)\.(\d)").unwrap();

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut variants: Vec<String> = Vec::new();

    // 1–3. Base variants for the full string
    add_with_derivatives(model, &dash_to_dot_re, &dot_to_dash_re, &mut seen, &mut variants);

    let parts: Vec<&str> = model.split('/').collect();

    // 4. Bare model variants (strip provider/aggregator prefix)
    if parts.len() >= 2 {
        let bare = parts[parts.len() - 1];
        add_with_derivatives(bare, &dash_to_dot_re, &dot_to_dash_re, &mut seen, &mut variants);
    }
    // Strip aggregator only (3+ parts):
    // "openrouter/anthropic/claude-opus-4.5" → "anthropic/claude-opus-4.5"
    if parts.len() >= 3 {
        let stripped = parts[1..].join("/");
        add_with_derivatives(&stripped, &dash_to_dot_re, &dot_to_dash_re, &mut seen, &mut variants);
    }

    // 5. Prepend known provider prefixes to bare variants
    let known_providers: [&str; 12] = [
        "anthropic", "openai", "google", "openrouter", "groq", "mistral",
        "xai", "cohere", "perplexity", "together", "fireworks", "deepseek",
    ];
    let bare_variants: Vec<String> = variants.iter().filter(|v| !v.contains('/')).cloned().collect();
    for v in bare_variants {
        for provider in known_providers.iter() {
            add(format!("{}/{}", provider, v), &mut seen, &mut variants);
        }
    }

    // Prepend aggregator to single-slash variants
    let single_slash_variants: Vec<String> = variants
        .iter()
        .filter(|v| v.matches('/').count() == 1)
        .cloned()
        .collect();
    let known_aggregators: [&str; 5] = ["openrouter", "opencode", "fireworks", "groq", "together"];
    for v in single_slash_variants {
        for agg in known_aggregators.iter() {
            add(format!("{}/{}", agg, v), &mut seen, &mut variants);
        }
    }

    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_efforts_parse() {
        for level in VALID_REASONING_EFFORTS {
            let r = parse_reasoning_effort(level).unwrap();
            assert_eq!(r, ReasoningConfig { enabled: true, effort: Some(level.to_string()) });
        }
    }

    #[test]
    fn disabled_aliases() {
        for alias in ["none", "false", "disabled"] {
            let r = parse_reasoning_effort(alias).unwrap();
            assert_eq!(r, ReasoningConfig { enabled: false, effort: None });
        }
        let r = parse_reasoning_effort(false).unwrap();
        assert_eq!(r, ReasoningConfig { enabled: false, effort: None });
    }

    #[test]
    fn unknown_and_empty_return_none() {
        assert_eq!(parse_reasoning_effort(""), None);
        assert_eq!(parse_reasoning_effort("   "), None);
        assert_eq!(parse_reasoning_effort("bogus"), None);
        assert_eq!(parse_reasoning_effort(true), None);
        assert_eq!(parse_reasoning_effort(None::<&str>), None);
    }

    #[test]
    fn case_and_whitespace_insensitive() {
        assert_eq!(
            parse_reasoning_effort("  HIGH ").unwrap(),
            ReasoningConfig { enabled: true, effort: Some("high".to_string()) }
        );
        assert_eq!(
            parse_reasoning_effort("Disabled").unwrap(),
            ReasoningConfig { enabled: false, effort: None }
        );
    }

    #[test]
    fn variants_exact_first() {
        let v = canonical_model_variants("claude-opus-4.5");
        assert_eq!(v[0], "claude-opus-4.5");
        assert_eq!(v.len(), 292, "golden count from upstream");
    }

    #[test]
    fn variants_contains_spelling_forms() {
        let v = canonical_model_variants("claude-opus-4.5");
        assert!(v.contains(&"claude-opus-4.5".to_string()));
        assert!(v.contains(&"claude-opus-4-5".to_string()));
        assert!(v.contains(&"claude.opus.4.5".to_string()));
        assert!(v.contains(&"claude.opus.4-5".to_string()));
        assert!(v.contains(&"anthropic/claude-opus-4.5".to_string()));
        assert!(v.contains(&"openrouter/claude-opus-4.5".to_string()));
    }

    #[test]
    fn variants_strip_provider() {
        let v = canonical_model_variants("openrouter/anthropic/claude-opus-4.5");
        assert!(v.contains(&"claude-opus-4.5".to_string()));
        assert!(v.contains(&"anthropic/claude-opus-4.5".to_string()));
    }

    #[test]
    fn variants_no_duplicates() {
        let v = canonical_model_variants("claude-opus");
        let mut seen = std::collections::HashSet::new();
        for s in &v {
            assert!(seen.insert(s.clone()), "duplicate {}", s);
        }
    }
}
