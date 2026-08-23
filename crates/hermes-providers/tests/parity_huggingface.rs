//! Source-derived parity oracle for
//! `plugins/model-providers/huggingface/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static HUGGINGFACE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn huggingface_profile_fields_and_aliases_match_upstream() {
    let _guard = HUGGINGFACE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile =
        get_provider_profile("huggingface").expect("Hugging Face profile must be registered");

    assert_eq!(profile.name, "huggingface");
    assert_eq!(profile.aliases, ["hf", "hugging-face", "huggingface-hub"]);
    assert_eq!(profile.env_vars, ["HF_TOKEN"]);
    assert_eq!(profile.display_name, "HuggingFace");
    assert_eq!(profile.description, "HuggingFace Inference API");
    assert_eq!(profile.signup_url, "https://huggingface.co/settings/tokens");
    assert_eq!(
        profile.fallback_models,
        ["Qwen/Qwen3.5-72B-Instruct", "deepseek-ai/DeepSeek-V3.2"]
    );
    assert_eq!(profile.base_url, "https://router.huggingface.co/v1");
    assert_eq!(profile.get_hostname(), "router.huggingface.co");
    assert_eq!(get_provider_profile("hf").unwrap().name, "huggingface");
    assert_eq!(
        get_provider_profile("hugging-face").unwrap().name,
        "huggingface"
    );
    assert_eq!(
        get_provider_profile("huggingface-hub").unwrap().name,
        "huggingface"
    );
}

#[test]
fn huggingface_is_listed_once_by_canonical_name() {
    let _guard = HUGGINGFACE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names.iter().filter(|name| *name == "huggingface").count(),
        1
    );
}
