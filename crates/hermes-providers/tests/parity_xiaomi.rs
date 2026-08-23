//! Source-derived parity oracle for
//! `plugins/model-providers/xiaomi/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static XIAOMI_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn xiaomi_profile_fields_and_aliases_match_upstream() {
    let _guard = XIAOMI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("xiaomi").expect("Xiaomi profile must be registered");

    assert_eq!(profile.name, "xiaomi");
    assert_eq!(profile.aliases, ["mimo", "xiaomi-mimo"]);
    assert_eq!(profile.env_vars, ["XIAOMI_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.xiaomimimo.com/v1");
    assert_eq!(profile.get_hostname(), "api.xiaomimimo.com");
    assert!(!profile.supports_health_check);
    assert!(profile.supports_vision);
    assert!(!profile.supports_vision_tool_messages);
    assert_eq!(get_provider_profile("mimo").unwrap().name, "xiaomi");
    assert_eq!(get_provider_profile("xiaomi-mimo").unwrap().name, "xiaomi");
}

#[test]
fn xiaomi_is_listed_once_by_canonical_name() {
    let _guard = XIAOMI_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "xiaomi").count(), 1);
}
