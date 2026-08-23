//! Source-derived parity oracle for
//! `plugins/model-providers/nvidia/__init__.py` @ b9aa928.
//!
//! The upstream profile's declarative fields are covered by the provider
//! profile and wiring tests; this focused test mirrors those cases while
//! checking registration and canonical-list behavior in Rust.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static NVIDIA_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn nvidia_profile_fields_and_aliases_match_upstream() {
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nvidia").expect("NVIDIA profile must be registered");

    assert_eq!(profile.name, "nvidia");
    assert_eq!(profile.aliases, ["nvidia-nim"]);
    assert_eq!(profile.env_vars, ["NVIDIA_API_KEY"]);
    assert_eq!(profile.display_name, "NVIDIA NIM");
    assert_eq!(profile.description, "NVIDIA NIM — accelerated inference");
    assert_eq!(profile.signup_url, "https://build.nvidia.com/");
    assert_eq!(
        profile.fallback_models,
        [
            "nvidia/llama-3.1-nemotron-70b-instruct",
            "nvidia/llama-3.3-70b-instruct"
        ]
    );
    assert_eq!(profile.base_url, "https://integrate.api.nvidia.com/v1");
    assert_eq!(profile.get_hostname(), "integrate.api.nvidia.com");
    assert_eq!(profile.default_max_tokens, Some(16_384));
    assert_eq!(get_provider_profile("nvidia-nim").unwrap().name, "nvidia");
}

#[test]
fn nvidia_is_listed_once_by_canonical_name() {
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "nvidia").count(), 1);
}
