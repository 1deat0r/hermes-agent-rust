//! Source-derived parity oracle for
//! `plugins/model-providers/gemini/__init__.py` @ b9aa928.
//!
//! The dedicated upstream profile test pins the auxiliary model; related
//! transport tests pin the thinking-config hook that the profile delegates to.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static GEMINI_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(model: &str, reasoning_config: Value, base_url: Option<&str>) -> Map<String, Value> {
    let mut context = Map::new();
    context.insert("model".into(), Value::String(model.into()));
    context.insert("reasoning_config".into(), reasoning_config);
    if let Some(base_url) = base_url {
        context.insert("base_url".into(), Value::String(base_url.into()));
    }
    context
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("JSON object")
}

#[test]
fn gemini_profile_fields_aliases_and_auxiliary_model_match_upstream() {
    let _guard = GEMINI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("gemini").expect("Gemini profile must be registered");

    assert_eq!(profile.name, "gemini");
    assert_eq!(
        profile.aliases,
        ["google", "google-gemini", "google-ai-studio"]
    );
    assert_eq!(profile.api_mode, "chat_completions");
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert_eq!(profile.env_vars, ["GOOGLE_API_KEY", "GEMINI_API_KEY"]);
    assert_eq!(
        profile.base_url,
        "https://generativelanguage.googleapis.com/v1beta"
    );
    assert_eq!(profile.auth_type, "api_key");
    assert!(profile.default_headers.is_empty());
    assert!(profile.fallback_models.is_empty());
    assert_eq!(profile.default_aux_model, "gemini-3.6-flash");
    assert!(profile.gemini_thinking);
    for alias in ["google", "google-gemini", "google-ai-studio"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "gemini");
    }

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "gemini").count(), 1);
}

#[test]
fn gemini_native_thinking_config_matches_transport_oracle() {
    let _guard = GEMINI_TEST_LOCK.lock().unwrap();
    let profile = get_provider_profile("gemini").expect("Gemini profile must be registered");

    let body = profile.build_extra_body(
        None,
        &context(
            "google/gemini-3-flash-preview",
            json!({"enabled": true, "effort": "high"}),
            None,
        ),
    );
    assert_eq!(
        body,
        object(json!({
            "thinking_config": {"includeThoughts": true, "thinkingLevel": "high"}
        }))
    );

    let disabled = profile.build_extra_body(
        None,
        &context(
            "gemini-3-pro-preview",
            json!({"enabled": false, "effort": "high"}),
            None,
        ),
    );
    assert_eq!(
        disabled,
        object(json!({"thinking_config": {"includeThoughts": false}}))
    );

    let non_gemini = profile.build_extra_body(
        None,
        &context(
            "gemma-3-27b-it",
            json!({"enabled": true, "effort": "high"}),
            None,
        ),
    );
    assert!(non_gemini.is_empty());

    let gemini_25 = profile.build_extra_body(
        None,
        &context(
            "gemini-2.5-flash",
            json!({"enabled": true, "effort": "high"}),
            None,
        ),
    );
    assert_eq!(
        gemini_25,
        object(json!({"thinking_config": {"includeThoughts": true}}))
    );
}

#[test]
fn gemini_openai_compat_uses_nested_snake_case_thinking_config() {
    let _guard = GEMINI_TEST_LOCK.lock().unwrap();
    let profile = get_provider_profile("gemini").expect("Gemini profile must be registered");
    let body = profile.build_extra_body(
        None,
        &context(
            "gemini-3-flash-preview",
            json!({"enabled": true, "effort": "high"}),
            Some("https://generativelanguage.googleapis.com/v1beta/openai/"),
        ),
    );

    assert_eq!(
        body,
        object(json!({
            "extra_body": {
                "google": {
                    "thinking_config": {
                        "include_thoughts": true,
                        "thinking_level": "high"
                    }
                }
            }
        }))
    );
}
