//! Source-derived parity oracle for
//! `plugins/model-providers/vertex/__init__.py` @ b9aa928.
//!
//! No dedicated upstream Vertex profile test module exists, so the pinned
//! source and its shared Gemini transport helper are the oracles. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static VERTEX_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(model: &str, reasoning_config: Value) -> Map<String, Value> {
    Map::from_iter([
        ("model".into(), Value::String(model.into())),
        ("reasoning_config".into(), reasoning_config),
    ])
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("JSON object")
}

#[test]
fn vertex_profile_fields_aliases_and_oauth_discovery_match_upstream() {
    let _guard = VERTEX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("vertex").expect("Vertex profile must be registered");

    assert_eq!(profile.name, "vertex");
    assert_eq!(
        profile.aliases,
        ["google-vertex", "vertex-ai", "gcp-vertex"]
    );
    assert_eq!(profile.api_mode, "chat_completions");
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert!(profile.env_vars.is_empty());
    assert_eq!(profile.base_url, "https://aiplatform.googleapis.com");
    assert_eq!(profile.auth_type, "vertex");
    assert!(profile.default_headers.is_empty());
    assert!(profile.fallback_models.is_empty());
    assert_eq!(profile.default_aux_model, "google/gemini-3.6-flash");
    assert!(profile.models_fetch_disabled);
    assert!(profile.vertex_thinking);
    for alias in ["google-vertex", "vertex-ai", "gcp-vertex"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "vertex");
    }

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "vertex").count(), 1);
}

#[test]
fn vertex_emits_nested_google_thinking_config_for_gemini_models() {
    let _guard = VERTEX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("vertex").unwrap();

    let body = profile.build_extra_body(
        None,
        &context(
            "gemini-3-flash-preview",
            json!({"enabled": true, "effort": "high"}),
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

    let disabled = profile.build_extra_body(
        None,
        &context(
            "gemini-3-pro-preview",
            json!({"enabled": false, "effort": "high"}),
        ),
    );
    assert_eq!(
        disabled,
        object(json!({
            "extra_body": {"google": {"thinking_config": {"include_thoughts": false}}}
        }))
    );

    let non_gemini = profile.build_extra_body(
        None,
        &context("gemma-3-27b-it", json!({"enabled": true, "effort": "high"})),
    );
    assert!(non_gemini.is_empty());
}

#[test]
fn vertex_disables_rest_model_discovery_before_network_access() {
    let _guard = VERTEX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("vertex").unwrap();

    // PARITY: VertexProfile.fetch_models() always returns None because the
    // setup wizard owns its curated model list rather than REST discovery.
    assert_eq!(
        profile.fetch_models(
            Some("oauth-access-token"),
            Some("http://127.0.0.1:1/should-not-connect"),
            8.0,
        ),
        None
    );
}
