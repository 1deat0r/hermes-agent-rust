//! Source-derived parity oracle for
//! `plugins/model-providers/ai-gateway/__init__.py` @ b9aa928.
//!
//! The profile module has no dedicated upstream test file; the pinned source
//! and related provider-routing tests are the oracle. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static AI_GATEWAY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn ai_gateway_profile_fields_aliases_headers_and_aux_model_match_upstream() {
    let _guard = AI_GATEWAY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile =
        get_provider_profile("ai-gateway").expect("AI Gateway profile must be registered");

    assert_eq!(profile.name, "ai-gateway");
    assert_eq!(
        profile.aliases,
        ["vercel", "vercel-ai-gateway", "ai_gateway", "aigateway"]
    );
    assert_eq!(profile.api_mode, "chat_completions");
    assert_eq!(profile.env_vars, ["AI_GATEWAY_API_KEY"]);
    assert_eq!(profile.base_url, "https://ai-gateway.vercel.sh/v1");
    assert_eq!(profile.get_hostname(), "ai-gateway.vercel.sh");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(
        profile
            .default_headers
            .get("HTTP-Referer")
            .map(String::as_str),
        Some("https://hermes-agent.nousresearch.com")
    );
    assert_eq!(
        profile.default_headers.get("X-Title").map(String::as_str),
        Some("Hermes Agent")
    );
    assert_eq!(profile.default_aux_model, "google/gemini-3-flash");
    assert!(profile.reasoning_passthrough);
}

#[test]
fn ai_gateway_reasoning_passthrough_matches_upstream_and_aliases_list_once() {
    let _guard = AI_GATEWAY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("ai-gateway").unwrap();

    let mut reasoning_config = Map::new();
    reasoning_config.insert("enabled".into(), Value::Bool(true));
    reasoning_config.insert("effort".into(), Value::String("high".into()));
    let context = Map::new();
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&reasoning_config), &context);
    assert_eq!(
        extra_body.get("reasoning"),
        Some(&Value::Object(reasoning_config.clone()))
    );
    assert!(top_level.is_empty());

    let (default_body, _) = profile.build_api_kwargs_extras(None, &context);
    assert_eq!(
        default_body.get("reasoning"),
        Some(&json!({"enabled": true, "effort": "medium"}))
    );

    let mut disabled_context = Map::new();
    disabled_context.insert("supports_reasoning".into(), Value::Bool(false));
    let (disabled_body, disabled_top_level) =
        profile.build_api_kwargs_extras(Some(&reasoning_config), &disabled_context);
    assert!(disabled_body.is_empty());
    assert!(disabled_top_level.is_empty());

    assert_eq!(get_provider_profile("vercel").unwrap().name, "ai-gateway");
    assert_eq!(
        get_provider_profile("vercel-ai-gateway").unwrap().name,
        "ai-gateway"
    );
    assert_eq!(
        get_provider_profile("ai_gateway").unwrap().name,
        "ai-gateway"
    );
    assert_eq!(
        get_provider_profile("aigateway").unwrap().name,
        "ai-gateway"
    );

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "ai-gateway").count(), 1);
}
