//! Source-derived parity oracle for
//! `plugins/model-providers/custom/__init__.py` @ b9aa928.
//!
//! Tier: unit. The context map is the Rust adapter for the upstream
//! keyword-only `ollama_num_ctx` argument and ignored model/context kwargs.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static CUSTOM_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(ollama_num_ctx: Option<i64>) -> Map<String, Value> {
    let mut context = Map::new();
    if let Some(ollama_num_ctx) = ollama_num_ctx {
        context.insert(
            "ollama_num_ctx".into(),
            Value::Number(ollama_num_ctx.into()),
        );
    }
    context
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("JSON object")
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

#[test]
fn custom_profile_fields_aliases_and_defaults_match_source() {
    let _guard = CUSTOM_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("custom").expect("Custom profile must be registered");

    assert_eq!(profile.name, "custom");
    assert_eq!(
        profile.aliases,
        [
            "ollama",
            "local",
            "vllm",
            "llamacpp",
            "llama.cpp",
            "llama-cpp"
        ]
    );
    assert_eq!(profile.api_mode, "chat_completions");
    assert!(profile.env_vars.is_empty());
    assert!(profile.base_url.is_empty());
    assert_eq!(profile.auth_type, "api_key");
    assert!(profile.fallback_models.is_empty());
    assert_eq!(profile.default_max_tokens, Some(65_536));
    assert!(profile.custom_provider);

    for alias in [
        "ollama",
        "local",
        "vllm",
        "llamacpp",
        "llama.cpp",
        "llama-cpp",
    ] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "custom");
    }
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "custom").count(), 1);
}

#[test]
fn custom_reasoning_disabled_and_none_emit_both_disable_fields() {
    let _guard = CUSTOM_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("custom").unwrap();

    let (no_config_body, no_config_top) = profile.build_api_kwargs_extras(None, &context(None));
    assert!(no_config_body.is_empty());
    assert!(no_config_top.is_empty());

    let empty_config = Map::new();
    let (empty_body, empty_top) =
        profile.build_api_kwargs_extras(Some(&empty_config), &context(None));
    assert!(empty_body.is_empty());
    assert!(empty_top.is_empty());

    for config in [
        reasoning(Some(false), None),
        reasoning(Some(true), Some("none")),
        reasoning(Some(false), Some("high")),
    ] {
        let (body, top) = profile.build_api_kwargs_extras(Some(&config), &context(None));
        assert_eq!(body, object(json!({"think": false})));
        assert_eq!(top, object(json!({"reasoning_effort": "none"})));
    }
}

#[test]
fn custom_enabled_effort_is_top_level_and_never_forces_think_true() {
    let _guard = CUSTOM_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("custom").unwrap();

    for (effort, expected) in [
        ("minimal", "minimal"),
        ("low", "low"),
        ("medium", "medium"),
        ("high", "high"),
        ("xhigh", "xhigh"),
        ("max", "max"),
        ("  HIGH  ", "high"),
    ] {
        let config = reasoning(Some(true), Some(effort));
        let (body, top) = profile.build_api_kwargs_extras(Some(&config), &context(None));
        assert!(body.is_empty());
        assert_eq!(
            top.get("reasoning_effort"),
            Some(&Value::String(expected.into()))
        );
        assert!(!body.contains_key("think"));
    }

    let no_effort = reasoning(Some(true), None);
    let (body, top) = profile.build_api_kwargs_extras(Some(&no_effort), &context(None));
    assert!(body.is_empty());
    assert!(top.is_empty());
}

#[test]
fn custom_num_ctx_is_orthogonal_and_composes_with_reasoning() {
    let _guard = CUSTOM_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("custom").unwrap();

    let (body, top) = profile.build_api_kwargs_extras(None, &context(Some(8_192)));
    assert_eq!(body, object(json!({"options": {"num_ctx": 8192}})));
    assert!(top.is_empty());

    let enabled = reasoning(Some(true), Some("high"));
    let (composed_body, composed_top) =
        profile.build_api_kwargs_extras(Some(&enabled), &context(Some(8_192)));
    assert_eq!(composed_body, object(json!({"options": {"num_ctx": 8192}})));
    assert_eq!(composed_top, object(json!({"reasoning_effort": "high"})));

    let (zero_body, zero_top) = profile.build_api_kwargs_extras(None, &context(Some(0)));
    assert!(zero_body.is_empty());
    assert!(zero_top.is_empty());
}
