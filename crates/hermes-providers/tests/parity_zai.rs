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
#[test]
fn zai_endpoint_table_matches_upstream_auth_lines_685_691() {
    let endpoints = hermes_providers::zai_endpoint_specs();
    assert_eq!(endpoints.len(), 4);
    assert_eq!(
        endpoints
            .iter()
            .map(|endpoint| (
                endpoint.id,
                endpoint.base_url,
                endpoint.models,
                endpoint.label,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "global",
                "https://api.z.ai/api/paas/v4",
                &["glm-5"][..],
                "Global",
            ),
            (
                "cn",
                "https://open.bigmodel.cn/api/paas/v4",
                &["glm-5"][..],
                "China",
            ),
            (
                "coding-global",
                "https://api.z.ai/api/coding/paas/v4",
                &["glm-5.2", "glm-5.1", "glm-5v-turbo", "glm-4.7"][..],
                "Global (Coding Plan)",
            ),
            (
                "coding-cn",
                "https://open.bigmodel.cn/api/coding/paas/v4",
                &["glm-5.2", "glm-5.1", "glm-5v-turbo", "glm-4.7"][..],
                "China (Coding Plan)",
            ),
        ]
    );
}

#[test]
fn zai_endpoint_probe_falls_back_through_models_in_order() {
    let endpoint = hermes_providers::zai_endpoint_specs()
        .iter()
        .find(|endpoint| endpoint.id == "coding-global")
        .unwrap();
    let mut attempted: Vec<String> = Vec::new();
    let result = hermes_providers::probe_zai_endpoint(endpoint, |_, model| {
        attempted.push(model.to_owned());
        model == "glm-4.7"
    });
    assert_eq!(
        attempted,
        ["glm-5.2", "glm-5.1", "glm-5v-turbo", "glm-4.7"].map(str::to_owned)
    );
    assert_eq!(result.unwrap().model, "glm-4.7");
}

#[test]
fn zai_endpoint_chooser_uses_priority_not_probe_completion_order() {
    let mut attempted: Vec<(&'static str, String)> = Vec::new();
    let result = hermes_providers::choose_zai_endpoint(|endpoint, model| {
        attempted.push((endpoint.id, model.to_owned()));
        matches!(endpoint.id, "cn" | "coding-global") && model == endpoint.models[0]
    });
    assert_eq!(result.unwrap().id, "cn");
    assert_eq!(
        attempted,
        vec![("global", "glm-5".into()), ("cn", "glm-5".into())]
    );
}

#[test]
fn zai_endpoint_chooser_returns_none_when_all_probes_fail() {
    assert!(hermes_providers::choose_zai_endpoint(|_, _| false).is_none());
}

#[test]
fn zai_base_url_precedence_matches_upstream_lines_784_801() {
    assert_eq!(
        hermes_providers::resolve_zai_base_url(
            "key",
            "https://default.example",
            "https://override.example",
            Some("https://cached.example"),
            Some("https://detected.example"),
        ),
        "https://override.example"
    );
    assert_eq!(
        hermes_providers::resolve_zai_base_url(
            "",
            "https://default.example",
            "",
            Some("https://cached.example"),
            Some("https://detected.example"),
        ),
        "https://default.example"
    );
    assert_eq!(
        hermes_providers::resolve_zai_base_url(
            "key",
            "https://default.example",
            "",
            Some("https://cached.example"),
            Some("https://detected.example"),
        ),
        "https://cached.example"
    );
    assert_eq!(
        hermes_providers::resolve_zai_base_url(
            "key",
            "https://default.example",
            "",
            None,
            Some("https://detected.example"),
        ),
        "https://detected.example"
    );
    assert_eq!(
        hermes_providers::resolve_zai_base_url("key", "https://default.example", "", None, None,),
        "https://default.example"
    );
}
