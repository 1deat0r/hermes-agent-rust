//! Source-derived parity oracle for
//! `plugins/model-providers/novita/__init__.py` @ b9aa928.
//!
//! The upstream provider test checks profile loading and the pricing-cache
//! helper; this focused unit covers the declarative profile contract while
//! the pricing helper remains a future hermes-cli seam.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static NOVITA_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn novita_profile_fields_and_aliases_match_upstream() {
    let _guard = NOVITA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("novita").expect("NovitaAI profile must be registered");

    assert_eq!(profile.name, "novita");
    assert_eq!(profile.aliases, ["novita-ai", "novitaai"]);
    assert_eq!(profile.display_name, "NovitaAI");
    assert_eq!(
        profile.description,
        "NovitaAI — AI-native cloud for builders and agents"
    );
    assert_eq!(
        profile.signup_url,
        "https://novita.ai/settings/key-management"
    );
    assert_eq!(profile.env_vars, ["NOVITA_API_KEY", "NOVITA_BASE_URL"]);
    assert_eq!(profile.base_url, "https://api.novita.ai/openai/v1");
    assert_eq!(profile.get_hostname(), "api.novita.ai");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.default_aux_model, "deepseek/deepseek-v3-0324");
    assert_eq!(
        profile.fallback_models,
        [
            "moonshotai/kimi-k2.5",
            "minimax/minimax-m2.7",
            "zai-org/glm-5",
            "deepseek/deepseek-v3-0324",
            "deepseek/deepseek-r1-0528",
            "qwen/qwen3-235b-a22b-fp8"
        ]
    );
    assert_eq!(get_provider_profile("novita-ai").unwrap().name, "novita");
    assert_eq!(get_provider_profile("novitaai").unwrap().name, "novita");
}

#[test]
fn novita_is_listed_once_by_canonical_name() {
    let _guard = NOVITA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "novita").count(), 1);
}
