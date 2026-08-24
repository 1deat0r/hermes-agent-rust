//! Source-derived parity oracle for
//! `plugins/model-providers/nous/__init__.py` @ b9aa928.
//!
//! The upstream profile is covered by provider-profile and chat-completions
//! tests; these cases mirror its declarative fields, Portal body hook, and
//! reasoning omission behavior. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static NOUS_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn nous_profile_fields_aliases_and_fallbacks_match_upstream() {
    let _guard = NOUS_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nous").expect("Nous must be registered");

    assert_eq!(profile.name, "nous");
    assert_eq!(profile.aliases, ["nous-portal", "nousresearch"]);
    assert_eq!(profile.display_name, "Nous Research");
    assert_eq!(profile.description, "Nous Research — Hermes model family");
    assert_eq!(profile.signup_url, "https://nousresearch.com/");
    assert_eq!(profile.env_vars, ["NOUS_API_KEY"]);
    assert_eq!(
        profile.base_url,
        "https://inference-api.nousresearch.com/v1"
    );
    assert_eq!(profile.auth_type, "oauth_device_code");
    assert_eq!(profile.default_max_tokens, None);
    assert_eq!(profile.fallback_models, ["hermes-3-405b", "hermes-3-70b"]);
    assert!(profile.default_aux_model.is_empty());
    assert!(profile.nous_portal);
    assert_eq!(get_provider_profile("nous-portal").unwrap().name, "nous");
    assert_eq!(get_provider_profile("nousresearch").unwrap().name, "nous");

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "nous").count(), 1);
}

#[test]
fn nous_portal_body_and_sticky_routing_match_upstream() {
    let _guard = NOUS_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nous").unwrap();

    let body = profile.build_extra_body(None, &Map::new());
    assert_eq!(
        body.get("tags"),
        Some(&json!([
            "product=hermes-agent",
            "client=hermes-client-v0.20.0"
        ]))
    );
    assert!(!body.contains_key("session_id"));

    let first = profile.build_extra_body(Some("cron_job42_20260801_090000"), &Map::new());
    let second = profile.build_extra_body(Some("cron_job42_20260802_090000"), &Map::new());
    assert_eq!(first.get("session_id"), Some(&json!("cron_job42")));
    assert_eq!(first.get("session_id"), second.get("session_id"));

    let ambient = Map::from_iter([(
        "conversation_context".into(),
        Value::String("root-conversation".into()),
    )]);
    let ambient_body = profile.build_extra_body(Some("segment-after-compaction"), &ambient);
    assert_eq!(
        ambient_body.get("session_id"),
        Some(&json!("root-conversation"))
    );
    assert_eq!(
        ambient_body.get("tags"),
        Some(&json!([
            "product=hermes-agent",
            "client=hermes-client-v0.20.0",
            "conversation=root-conversation"
        ]))
    );

    let mut with_preferences = Map::new();
    with_preferences.insert("provider_preferences".into(), json!({"only": ["nous"]}));
    let preferred = profile.build_extra_body(Some("session-1"), &with_preferences);
    assert_eq!(preferred.get("session_id"), Some(&json!("session-1")));
    assert_eq!(preferred.get("provider"), Some(&json!({"only": ["nous"]})));
}

#[test]
fn nous_reasoning_passthrough_and_disabled_omission_match_upstream() {
    let _guard = NOUS_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nous").unwrap();

    let mut supports_reasoning = Map::new();
    supports_reasoning.insert("supports_reasoning".into(), Value::Bool(true));
    let (default_body, default_top) = profile.build_api_kwargs_extras(None, &supports_reasoning);
    assert_eq!(
        default_body.get("reasoning"),
        Some(&json!({"enabled": true, "effort": "medium"}))
    );
    assert!(default_top.is_empty());

    let reasoning = Map::from_iter([
        ("enabled".into(), Value::Bool(true)),
        ("effort".into(), Value::String("high".into())),
        (
            "budget".into(),
            Value::Number(serde_json::Number::from(4096)),
        ),
    ]);
    let (configured_body, configured_top) =
        profile.build_api_kwargs_extras(Some(&reasoning), &supports_reasoning);
    assert_eq!(
        configured_body.get("reasoning"),
        Some(&Value::Object(reasoning.clone()))
    );
    assert!(configured_top.is_empty());

    let disabled = Map::from_iter([("enabled".into(), Value::Bool(false))]);
    let (disabled_body, disabled_top) =
        profile.build_api_kwargs_extras(Some(&disabled), &supports_reasoning);
    assert!(disabled_body.is_empty());
    assert!(disabled_top.is_empty());

    let unsupported = Map::new();
    let (unsupported_body, unsupported_top) =
        profile.build_api_kwargs_extras(Some(&reasoning), &unsupported);
    assert!(unsupported_body.is_empty());
    assert!(unsupported_top.is_empty());
}
