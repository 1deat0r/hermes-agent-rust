//! Source-derived parity oracle for
//! `plugins/model-providers/upstage/__init__.py` @ b9aa928.
//!
//! The upstream profile is covered by the dedicated Upstage profile tests;
//! these cases mirror its declarative fields and Solar reasoning hook. Tier:
//! unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{Map, Value};

static UPSTAGE_TEST_LOCK: Mutex<()> = Mutex::new(());

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

#[test]
fn upstage_profile_fields_aliases_and_fallback_match_source() {
    let _guard = UPSTAGE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("upstage").expect("Upstage must be registered");

    assert_eq!(profile.name, "upstage");
    assert_eq!(profile.api_mode, "chat_completions");
    assert_eq!(profile.aliases, ["solar"]);
    assert_eq!(profile.display_name, "Upstage Solar");
    assert_eq!(profile.description, "Upstage (Solar API)");
    assert_eq!(profile.signup_url, "https://console.upstage.ai/api-keys");
    assert_eq!(profile.env_vars, ["UPSTAGE_API_KEY", "UPSTAGE_BASE_URL"]);
    assert_eq!(profile.base_url, "https://api.upstage.ai/v1");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.fallback_models, ["solar-pro3"]);
    assert!(profile.default_aux_model.is_empty());
    assert!(profile.upstage_reasoning);
    assert_eq!(profile.get_hostname(), "api.upstage.ai");
    assert_eq!(get_provider_profile("solar").unwrap().name, "upstage");

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "upstage").count(), 1);
}

#[test]
fn upstage_reasoning_effort_mapping_matches_source() {
    let _guard = UPSTAGE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("upstage").unwrap();

    for (effort, expected) in [
        ("low", Some("low")),
        ("medium", Some("medium")),
        ("high", Some("high")),
        ("  HIGH  ", Some("high")),
        ("xhigh", Some("high")),
        ("max", Some("high")),
        ("ultra", Some("high")),
        ("hyperthink", Some("high")),
        ("none", Some("high")),
        ("minimal", None),
    ] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) =
            profile.build_api_kwargs_extras(Some(&config), &context(Some("solar-pro3")));
        assert!(extra_body.is_empty());
        match expected {
            Some(expected) => assert_eq!(
                top_level.get("reasoning_effort"),
                Some(&Value::String(expected.into()))
            ),
            None => assert!(top_level.is_empty()),
        }
    }

    let empty_config = Map::new();
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&empty_config), &context(Some("solar-pro3")));
    assert!(extra_body.is_empty());
    assert_eq!(
        top_level.get("reasoning_effort"),
        Some(&Value::String("medium".into()))
    );

    let enabled_without_effort = reasoning(Some(true), None);
    let (extra_body, top_level) = profile
        .build_api_kwargs_extras(Some(&enabled_without_effort), &context(Some("solar-pro3")));
    assert!(extra_body.is_empty());
    assert_eq!(
        top_level.get("reasoning_effort"),
        Some(&Value::String("medium".into()))
    );
}

#[test]
fn upstage_no_config_defaults_on_and_explicit_disable_omits_field() {
    let _guard = UPSTAGE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("upstage").unwrap();

    for model in [
        Some("solar-pro3"),
        Some("solar-pro"),
        Some("solar-open2"),
        None,
    ] {
        let (extra_body, top_level) = profile.build_api_kwargs_extras(None, &context(model));
        assert!(extra_body.is_empty());
        assert_eq!(
            top_level.get("reasoning_effort"),
            Some(&Value::String("medium".into()))
        );
    }

    let disabled = reasoning(Some(false), Some("high"));
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&disabled), &context(Some("solar-pro3")));
    assert!(extra_body.is_empty());
    assert!(top_level.is_empty());
}

#[test]
fn upstage_deny_listed_models_never_receive_reasoning_effort() {
    let _guard = UPSTAGE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("upstage").unwrap();
    let enabled = reasoning(Some(true), Some("high"));

    for model in [
        "solar-mini",
        "solar-mini-202610",
        "syn-pro",
        "vendor/syn-pro-v2",
        "SOLAR-MINI-250127",
    ] {
        let (extra_body, top_level) =
            profile.build_api_kwargs_extras(Some(&enabled), &context(Some(model)));
        assert!(extra_body.is_empty());
        assert!(top_level.is_empty());
    }

    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&enabled), &context(Some("future-solar-model")));
    assert!(extra_body.is_empty());
    assert_eq!(
        top_level.get("reasoning_effort"),
        Some(&Value::String("high".into()))
    );
}
