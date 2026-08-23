//! Source-derived parity oracle for
//! `plugins/model-providers/deepseek/__init__.py` @ b9aa928.
//!
//! The upstream profile is exercised by provider wiring/transport tests; this
//! mirrors its declarative fields and `build_api_kwargs_extras` cases. Tier:
//! unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static DEEPSEEK_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(model: Option<&str>) -> Map<String, Value> {
    model.map_or_else(Map::new, |model| {
        Map::from_iter([("model".into(), Value::String(model.into()))])
    })
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("JSON object")
}

#[test]
fn deepseek_profile_fields_aliases_and_fallbacks_match_upstream() {
    let _guard = DEEPSEEK_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("deepseek").expect("DeepSeek must be registered");

    assert_eq!(profile.name, "deepseek");
    assert_eq!(profile.aliases, ["deepseek-chat"]);
    assert_eq!(profile.display_name, "DeepSeek");
    assert_eq!(profile.description, "DeepSeek — native DeepSeek API");
    assert_eq!(profile.signup_url, "https://platform.deepseek.com/");
    assert_eq!(profile.env_vars, ["DEEPSEEK_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.deepseek.com/v1");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.default_max_tokens, None);
    assert_eq!(
        profile.fallback_models,
        ["deepseek-v4-pro", "deepseek-v4-flash"]
    );
    assert_eq!(profile.default_aux_model, "deepseek-v4-flash");
    assert!(profile.deepseek_reasoning);
    assert_eq!(
        get_provider_profile("deepseek-chat").unwrap().name,
        "deepseek"
    );
    assert!(get_provider_profile("deepseek-reasoner").is_none());

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "deepseek").count(), 1);
}

#[test]
fn deepseek_reasoning_wire_shape_and_model_gating_match_upstream() {
    let _guard = DEEPSEEK_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("deepseek").unwrap();

    let (no_config_body, no_config_top) =
        profile.build_api_kwargs_extras(None, &context(Some("deepseek-v4-flash")));
    assert_eq!(
        no_config_body,
        object(json!({"thinking": {"type": "enabled"}}))
    );
    assert!(no_config_top.is_empty());

    let disabled_config = Map::from_iter([
        ("enabled".into(), Value::Bool(false)),
        ("effort".into(), Value::String("high".into())),
    ]);
    let (disabled_body, disabled_top) =
        profile.build_api_kwargs_extras(Some(&disabled_config), &context(Some("deepseek-v4-pro")));
    assert_eq!(
        disabled_body,
        object(json!({"thinking": {"type": "disabled"}}))
    );
    assert!(disabled_top.is_empty());

    for (effort, expected) in [
        ("low", "low"),
        ("medium", "medium"),
        ("high", "high"),
        ("xhigh", "max"),
        ("max", "max"),
        ("ultra", "max"),
    ] {
        let reasoning = Map::from_iter([
            ("enabled".into(), Value::Bool(true)),
            ("effort".into(), Value::String(effort.into())),
        ]);
        let (body, top) =
            profile.build_api_kwargs_extras(Some(&reasoning), &context(Some("deepseek-v4-flash")));
        assert_eq!(body, object(json!({"thinking": {"type": "enabled"}})));
        assert_eq!(
            top.get("reasoning_effort"),
            Some(&Value::String(expected.into()))
        );
    }

    let unknown_effort = Map::from_iter([("effort".into(), Value::String("none".into()))]);
    let (unknown_body, unknown_top) =
        profile.build_api_kwargs_extras(Some(&unknown_effort), &context(Some("deepseek-v4-flash")));
    assert_eq!(
        unknown_body,
        object(json!({"thinking": {"type": "enabled"}}))
    );
    assert!(unknown_top.is_empty());

    let v3 = Map::from_iter([
        ("enabled".into(), Value::Bool(true)),
        ("effort".into(), Value::String("high".into())),
    ]);
    let (v3_body, v3_top) =
        profile.build_api_kwargs_extras(Some(&v3), &context(Some("deepseek-v3.2")));
    assert!(v3_body.is_empty());
    assert!(v3_top.is_empty());

    let (unknown_model_body, unknown_model_top) =
        profile.build_api_kwargs_extras(Some(&v3), &context(Some("custom-model")));
    assert!(unknown_model_body.is_empty());
    assert!(unknown_model_top.is_empty());
}
