//! Parity oracle for
//! `plugins/model-providers/fireworks/__init__.py` @ b9aa928 and
//! `tests/plugins/model_providers/test_fireworks_profile.py`.
//!
//! Tier: unit. CLI/provider-resolution tests remain future-crate oracles.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static FIREWORKS_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn fireworks_profile_identity_headers_and_models_match_upstream() {
    let _guard = FIREWORKS_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("fireworks").expect("Fireworks profile must be registered");

    assert_eq!(profile.name, "fireworks");
    assert_eq!(profile.aliases, ["fireworks-ai", "fw"]);
    assert_eq!(profile.display_name, "Fireworks AI");
    assert_eq!(
        profile.description,
        "Fireworks AI — OpenAI-compatible direct model API"
    );
    assert_eq!(
        profile.signup_url,
        "https://app.fireworks.ai/settings/users/api-keys"
    );
    assert_eq!(profile.env_vars, ["FIREWORKS_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.fireworks.ai/inference/v1");
    assert_eq!(profile.get_hostname(), "api.fireworks.ai");
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
    assert_eq!(
        profile
            .default_headers
            .get("User-Agent")
            .map(String::as_str),
        Some("HermesAgent/0.20.0")
    );
    assert_eq!(
        profile.default_aux_model,
        "accounts/fireworks/models/glm-5p2"
    );
}

#[test]
fn fireworks_aliases_and_payg_fallbacks_are_registered_once() {
    let _guard = FIREWORKS_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("fireworks").unwrap();

    assert_eq!(
        get_provider_profile("fireworks-ai").unwrap().name,
        "fireworks"
    );
    assert_eq!(get_provider_profile("fw").unwrap().name, "fireworks");
    assert_eq!(
        profile.fallback_models,
        [
            "accounts/fireworks/models/kimi-k2p6",
            "accounts/fireworks/models/glm-5p2",
            "accounts/fireworks/models/kimi-k2p7-code"
        ]
    );
    assert!(profile
        .fallback_models
        .iter()
        .all(|model| model.starts_with("accounts/fireworks/models/")));
    assert!(
        profile
            .fallback_models
            .iter()
            .all(|model| !model.contains("/routers/")
                && !model.to_ascii_lowercase().contains("turbo"))
    );

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "fireworks").count(), 1);
}
