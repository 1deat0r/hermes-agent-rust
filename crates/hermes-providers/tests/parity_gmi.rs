//! Source-derived parity oracle for
//! `plugins/model-providers/gmi/__init__.py` @ b9aa928.
//!
//! The upstream GMI tests cover aliases, profile loading, client attribution,
//! and auxiliary routing; this focused unit covers the declarative profile
//! contract. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static GMI_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn gmi_profile_fields_aliases_headers_and_models_match_upstream() {
    let _guard = GMI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("gmi").expect("GMI Cloud profile must be registered");

    assert_eq!(profile.name, "gmi");
    assert_eq!(profile.aliases, ["gmi-cloud", "gmicloud"]);
    assert_eq!(profile.display_name, "GMI Cloud");
    assert_eq!(
        profile.description,
        "GMI Cloud — multi-model direct API (slash-form model IDs)"
    );
    assert_eq!(profile.signup_url, "https://www.gmicloud.ai/");
    assert_eq!(profile.env_vars, ["GMI_API_KEY", "GMI_BASE_URL"]);
    assert_eq!(profile.base_url, "https://api.gmi-serving.com/v1");
    assert_eq!(profile.get_hostname(), "api.gmi-serving.com");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(
        profile
            .default_headers
            .get("User-Agent")
            .map(String::as_str),
        Some("HermesAgent/0.20.0")
    );
    assert_eq!(
        profile.default_aux_model,
        "google/gemini-3.1-flash-lite-preview"
    );
    assert_eq!(
        profile.fallback_models,
        [
            "zai-org/GLM-5.1-FP8",
            "deepseek-ai/DeepSeek-V3.2",
            "moonshotai/Kimi-K2.5",
            "google/gemini-3.1-flash-lite-preview",
            "anthropic/claude-sonnet-5",
            "anthropic/claude-sonnet-4.6",
            "openai/gpt-5.4"
        ]
    );
    assert_eq!(get_provider_profile("gmi-cloud").unwrap().name, "gmi");
    assert_eq!(get_provider_profile("gmicloud").unwrap().name, "gmi");
}

#[test]
fn gmi_is_listed_once_by_canonical_name() {
    let _guard = GMI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "gmi").count(), 1);
}
