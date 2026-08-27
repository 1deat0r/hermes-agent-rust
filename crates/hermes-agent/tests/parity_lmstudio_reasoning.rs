// Tier: unit — mirrors tests/agent/test_lmstudio_reasoning.py.

use hermes_agent::lmstudio_reasoning::resolve_lmstudio_effort;
use hermes_constants::VALID_REASONING_EFFORTS;
use serde_json::{json, Map, Value};

fn cfg(value: Value) -> Map<String, Value> {
    value.as_object().expect("object config").clone()
}

/// Rank of each value LM Studio accepts, weakest to strongest — the same
/// `_LM_RANK` table the upstream test uses to detect a ladder inversion.
fn rank(effort: &str) -> usize {
    match effort {
        "minimal" => 0,
        "low" => 1,
        "medium" => 2,
        "high" => 3,
        "xhigh" => 4,
        other => panic!("unranked LM Studio effort {other}"),
    }
}

#[test]
fn effort_ladder_is_monotonic() {
    let resolved: Vec<String> = VALID_REASONING_EFFORTS
        .iter()
        .map(|effort| {
            resolve_lmstudio_effort(Some(&cfg(json!({"enabled": true, "effort": effort}))), None)
                .unwrap_or_else(|| panic!("{effort} must resolve"))
        })
        .collect();
    let ranks: Vec<usize> = resolved.iter().map(|value| rank(value)).collect();
    assert_eq!(ranks, {
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        sorted
    });
    assert_eq!(
        resolved,
        vec!["minimal", "low", "medium", "high", "xhigh", "xhigh", "xhigh"]
    );
}

#[test]
fn clamped_effort_is_still_checked_against_allowed_options() {
    for effort in ["max", "ultra"] {
        let config = cfg(json!({"enabled": true, "effort": effort}));
        assert_eq!(
            resolve_lmstudio_effort(Some(&config), Some(&["off", "minimal", "low"])),
            None
        );
        assert_eq!(
            resolve_lmstudio_effort(Some(&config), Some(&["low", "medium", "high", "xhigh"])),
            Some("xhigh".to_string())
        );
    }
}

#[test]
fn clamp_does_not_rewrite_published_allowed_options() {
    let config = cfg(json!({"enabled": true, "effort": "max"}));
    assert_eq!(resolve_lmstudio_effort(Some(&config), Some(&["max"])), None);
}

// Source-derived branches the upstream oracle does not exercise.
#[test]
fn missing_or_empty_config_uses_the_medium_default() {
    assert_eq!(resolve_lmstudio_effort(None, None), Some("medium".into()));
    // Python truthiness: an empty config dict is falsy, so no branch is read.
    assert_eq!(
        resolve_lmstudio_effort(Some(&Map::new()), None),
        Some("medium".into())
    );
}

#[test]
fn disabled_reasoning_maps_to_none_effort() {
    let disabled = cfg(json!({"enabled": false}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&disabled), None),
        Some("none".into())
    );
    // `enabled is False` is an identity test: an explicit null is not False,
    // so the effort branch runs instead.
    let null_enabled = cfg(json!({"enabled": null, "effort": "high"}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&null_enabled), None),
        Some("high".into())
    );
    // ... and a disabled config wins over any effort value.
    let both = cfg(json!({"enabled": false, "effort": "ultra"}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&both), None),
        Some("none".into())
    );
}

#[test]
fn toggle_aliases_and_normalization_apply_to_the_request() {
    for (effort, expected) in [
        ("off", "none"),
        ("ON", "medium"),
        ("  Low  ", "low"),
        ("unheard-of", "medium"),
        ("", "medium"),
    ] {
        let config = cfg(json!({"enabled": true, "effort": effort}));
        assert_eq!(
            resolve_lmstudio_effort(Some(&config), None),
            Some(expected.to_string()),
            "effort {effort:?}"
        );
    }
    // A non-string effort is falsy-or-not per Python `or ""` semantics; a
    // missing key takes the same path.
    let non_string = cfg(json!({"enabled": true, "effort": null}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&non_string), None),
        Some("medium".into())
    );
}

#[test]
fn empty_allowed_options_skip_the_clamp_check() {
    let config = cfg(json!({"enabled": true, "effort": "xhigh"}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&config), Some(&[])),
        Some("xhigh".into())
    );
}

#[test]
fn allowed_options_are_alias_mapped_verbatim_without_clamping() {
    // Published "on" normalizes to "medium"; a published "max" stays "max", so
    // the clamp map never rewrites the model's own vocabulary.
    let config = cfg(json!({"enabled": true, "effort": "medium"}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&config), Some(&["off", "on"])),
        Some("medium".into())
    );
    let on_config = cfg(json!({"enabled": true, "effort": "on"}));
    assert_eq!(
        resolve_lmstudio_effort(Some(&on_config), Some(&["off"])),
        None,
        "resolved medium is not in the published {{off}} set"
    );
}
