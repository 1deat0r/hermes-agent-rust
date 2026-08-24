//! Source-derived parity oracle for
//! `plugins/model-providers/qwen-oauth/__init__.py` @ b9aa928.
//!
//! The upstream profile is covered by provider-profile, profile-wiring, and
//! chat-completions transport tests. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static QWEN_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn qwen_profile_fields_aliases_and_registration_match_source() {
    let _guard = QWEN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("qwen-oauth").expect("Qwen must be registered");

    assert_eq!(profile.name, "qwen-oauth");
    assert_eq!(profile.aliases, ["qwen", "qwen-portal", "qwen-cli"]);
    assert_eq!(profile.env_vars, ["QWEN_API_KEY"]);
    assert_eq!(profile.base_url, "https://portal.qwen.ai/v1");
    assert_eq!(profile.auth_type, "oauth_external");
    assert_eq!(profile.default_max_tokens, Some(65_536));
    assert!(profile.qwen_portal);

    for alias in ["qwen", "qwen-portal", "qwen-cli"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "qwen-oauth");
    }
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "qwen-oauth").count(), 1);
}

#[test]
fn qwen_messages_normalize_and_protect_nested_image_retry_mutation() {
    let _guard = QWEN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("qwen-oauth").unwrap();

    let messages = vec![
        json!({"role": "system", "content": "Be helpful"}),
        json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "see image"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,original"}}
            ]
        }),
    ];

    let mut prepared = profile.prepare_messages(&messages);
    assert_eq!(
        prepared[0]["content"],
        json!([{"type": "text", "text": "Be helpful", "cache_control": {"type": "ephemeral"}}])
    );
    assert_eq!(messages[0]["content"], json!("Be helpful"));
    assert_eq!(
        prepared[1]["content"][0],
        json!({"type": "text", "text": "see image"})
    );
    assert_eq!(
        prepared[1]["content"][1]["image_url"]["url"],
        json!("data:image/png;base64,original")
    );

    prepared[1]["content"][1]["image_url"]["url"] = json!("data:image/png;base64,shrunk");
    assert_eq!(
        messages[1]["content"][1]["image_url"]["url"],
        json!("data:image/png;base64,original")
    );

    let mixed = vec![json!({
        "role": "user",
        "content": ["hello", {"type": "text", "text": "world"}, 7]
    })];
    let normalized = profile.prepare_messages(&mixed);
    assert_eq!(
        normalized[0]["content"],
        json!([{"type": "text", "text": "hello"}, {"type": "text", "text": "world"}])
    );
}

#[test]
fn qwen_extra_body_enables_high_resolution_images() {
    let _guard = QWEN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("qwen-oauth").unwrap();

    let body = profile.build_extra_body(None, &Map::new());
    assert_eq!(
        body.get("vl_high_resolution_images"),
        Some(&Value::Bool(true))
    );
}

#[test]
fn qwen_metadata_is_top_level_and_empty_metadata_is_omitted() {
    let _guard = QWEN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("qwen-oauth").unwrap();

    let metadata = json!({"sessionId": "s123", "promptId": "p456"});
    let mut context = Map::new();
    context.insert("qwen_session_metadata".into(), metadata.clone());
    let (extra_body, top_level) = profile.build_api_kwargs_extras(None, &context);
    assert!(extra_body.is_empty());
    assert_eq!(top_level.get("metadata"), Some(&metadata));

    context.insert("qwen_session_metadata".into(), json!({}));
    let (extra_body, top_level) = profile.build_api_kwargs_extras(None, &context);
    assert!(extra_body.is_empty());
    assert!(top_level.is_empty());
}
