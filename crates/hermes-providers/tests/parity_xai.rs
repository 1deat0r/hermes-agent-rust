//! Source-derived parity oracle for
//! `plugins/model-providers/xai/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static XAI_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn xai_profile_fields_and_aliases_match_upstream() {
    let _guard = XAI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("xai").expect("xAI profile must be registered");

    assert_eq!(profile.name, "xai");
    assert_eq!(profile.aliases, ["grok", "x-ai", "x.ai"]);
    assert_eq!(profile.api_mode, "codex_responses");
    assert_eq!(profile.env_vars, ["XAI_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.x.ai/v1");
    assert_eq!(profile.get_hostname(), "api.x.ai");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(
        profile.default_headers.get("User-Agent"),
        Some(&"Hermes-Agent/0.20.0".to_owned())
    );
    assert_eq!(profile.default_headers.len(), 1);
    assert_eq!(get_provider_profile("grok").unwrap().name, "xai");
    assert_eq!(get_provider_profile("x-ai").unwrap().name, "xai");
    assert_eq!(get_provider_profile("x.ai").unwrap().name, "xai");
}

#[test]
fn xai_is_listed_once_by_canonical_name() {
    let _guard = XAI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "xai").count(), 1);
}
