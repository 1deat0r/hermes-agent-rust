//! Parity oracles for plugins/model-providers/zai/__init__.py at upstream
//! commit b9aa928.
//!
//! Tier: unit. The tests exercise the registered profile and its reasoning
//! hook without contacting Z.AI.

use hermes_providers::base::FixedTemperature;
use hermes_providers::registry::get_provider_profile;
use hermes_providers::ProviderProfile;
use serde_json::{Map, Value};

fn context(model: Option<&str>) -> Map<String, Value> {
    let mut context = Map::new();
    if let Some(model) = model {
        context.insert("model".into(), Value::String(model.into()));
    }
    context
}

fn reasoning(enabled: Option<bool>, effort: Option<&str>) -> Map<String, Value> {
    let mut config = Map::new();
    if let Some(enabled) = enabled {
        config.insert("enabled".into(), Value::Bool(enabled));
    }
    if let Some(effort) = effort {
        config.insert("effort".into(), Value::String(effort.into()));
    }
    config
}

fn thinking(kind: &str) -> Value {
    serde_json::json!({"type": kind})
}

#[test]
fn zai_profile_fields_aliases_and_fallbacks_match_source() {
    let profile = get_provider_profile("zai").expect("Z.AI profile must be registered");
    assert_eq!(profile.name, "zai");
    assert_eq!(profile.aliases, ["glm", "z-ai", "z.ai", "zhipu"]);
    assert_eq!(profile.display_name, "Z.AI (GLM)");
    assert_eq!(profile.description, "Z.AI / GLM — Zhipu AI models");
    assert_eq!(profile.signup_url, "https://z.ai/");
    assert_eq!(
        profile.env_vars,
        ["GLM_API_KEY", "ZAI_API_KEY", "Z_AI_API_KEY"]
    );
    assert_eq!(profile.base_url, "https://api.z.ai/api/paas/v4");
    assert_eq!(profile.fallback_models, ["glm-5.2", "glm-5", "glm-4-9b"]);
    assert_eq!(profile.default_aux_model, "glm-4.5-flash");
    assert_eq!(profile.fixed_temperature, FixedTemperature::CallerDefault);
    assert!(profile.zai_reasoning);
    assert_eq!(profile.get_hostname(), "api.z.ai");

    for alias in ["glm", "z-ai", "z.ai", "zhipu"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "zai");
    }
}

#[test]
fn zai_thinking_gating_matches_glm_version_predicate() {
    let profile = get_provider_profile("zai").unwrap();

    let (extra_body, top_level) = profile.build_api_kwargs_extras(None, &context(Some("glm-5")));
    assert!(extra_body.is_empty());
    assert!(top_level.is_empty());

    let enabled = reasoning(Some(true), Some("medium"));
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&enabled), &context(Some("glm-5")));
    assert_eq!(extra_body.get("thinking"), Some(&thinking("enabled")));
    assert!(top_level.is_empty());

    let disabled = reasoning(Some(false), None);
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&disabled), &context(Some("glm-5")));
    assert_eq!(extra_body.get("thinking"), Some(&thinking("disabled")));
    assert!(top_level.is_empty());

    for model in [
        "glm-4.5",
        "glm-4.5-air",
        "glm-4.5-flash",
        "glm-4.6",
        "glm-5",
        "glm-5.2",
        "GLM-5",
    ] {
        let (extra_body, _) =
            profile.build_api_kwargs_extras(Some(&disabled), &context(Some(model)));
        assert_eq!(extra_body.get("thinking"), Some(&thinking("disabled")));
    }

    for model in ["glm-4-9b", "glm-4.4", "", "vendor/glm-4.5"] {
        let (extra_body, top_level) =
            profile.build_api_kwargs_extras(Some(&disabled), &context(Some(model)));
        assert!(extra_body.is_empty(), "unexpected thinking for {model:?}");
        assert!(top_level.is_empty(), "unexpected effort for {model:?}");
    }

    let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&disabled), &Map::new());
    assert!(extra_body.is_empty());
    assert!(top_level.is_empty());
}

#[test]
fn zai_glm_52_effort_mapping_matches_source() {
    let profile = get_provider_profile("zai").unwrap();

    for (effort, expected) in [
        ("high", Some("high")),
        ("low", Some("high")),
        ("medium", Some("high")),
        ("minimal", Some("high")),
        ("xhigh", Some("max")),
        ("max", Some("max")),
        ("ultra", Some("max")),
        ("none", None),
        ("", None),
    ] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) =
            profile.build_api_kwargs_extras(Some(&config), &context(Some("glm-5.2")));
        assert_eq!(extra_body.get("thinking"), Some(&thinking("enabled")));
        match expected {
            Some(expected) => assert_eq!(
                top_level.get("reasoning_effort"),
                Some(&Value::String(expected.into()))
            ),
            None => assert!(top_level.is_empty()),
        }
    }

    let disabled = reasoning(Some(false), Some("high"));
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&disabled), &context(Some("glm-5.2")));
    assert_eq!(extra_body.get("thinking"), Some(&thinking("disabled")));
    assert!(top_level.is_empty());
}

#[test]
fn zai_glm_52_aliases_and_non_glm_52_models_match_source() {
    let profile = get_provider_profile("zai").unwrap();

    for model in [
        "z-ai/glm-5.2",
        "glm-5-2",
        "glm-5p2",
        "accounts/fireworks/models/glm-5p2",
        "zai-org-glm-5-2",
    ] {
        let config = reasoning(Some(true), Some("max"));
        let (extra_body, top_level) =
            profile.build_api_kwargs_extras(Some(&config), &context(Some(model)));
        assert_eq!(extra_body.get("thinking"), Some(&thinking("enabled")));
        assert_eq!(
            top_level.get("reasoning_effort"),
            Some(&Value::String("max".into()))
        );
    }

    for model in ["glm-5.1", "glm-5", "glm-4.7", "glm-4-9b", ""] {
        let config = reasoning(Some(true), Some("high"));
        let (_, top_level) = profile.build_api_kwargs_extras(Some(&config), &context(Some(model)));
        assert!(top_level.is_empty(), "unexpected effort for {model:?}");
    }
}

#[test]
fn provider_profile_default_is_not_zai_capable() {
    let profile = ProviderProfile::new("test");
    assert!(!profile.zai_reasoning);
}
