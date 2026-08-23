//! Source-derived parity oracle for
//! `plugins/model-providers/arcee/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static ARCEE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn arcee_profile_fields_and_aliases_match_upstream() {
    let _guard = ARCEE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("arcee").expect("Arcee profile must be registered");

    assert_eq!(profile.name, "arcee");
    assert_eq!(profile.aliases, ["arcee-ai", "arceeai"]);
    assert_eq!(profile.env_vars, ["ARCEEAI_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.arcee.ai/api/v1");
    assert_eq!(profile.get_hostname(), "api.arcee.ai");
    assert_eq!(get_provider_profile("arcee-ai").unwrap().name, "arcee");
    assert_eq!(get_provider_profile("arceeai").unwrap().name, "arcee");
}

#[test]
fn arcee_is_listed_once_by_canonical_name() {
    let _guard = ARCEE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "arcee").count(), 1);
}
