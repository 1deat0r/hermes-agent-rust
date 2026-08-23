//! Source-derived parity oracle for
//! `plugins/model-providers/kilocode/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static KILOCODE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn kilocode_profile_fields_and_aliases_match_upstream() {
    let _guard = KILOCODE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("kilocode").expect("Kilo Code profile must be registered");

    assert_eq!(profile.name, "kilocode");
    assert_eq!(profile.aliases, ["kilo-code", "kilo", "kilo-gateway"]);
    assert_eq!(profile.env_vars, ["KILOCODE_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.kilo.ai/api/gateway");
    assert_eq!(profile.get_hostname(), "api.kilo.ai");
    assert_eq!(profile.default_aux_model, "google/gemini-3.6-flash");
    assert_eq!(get_provider_profile("kilo-code").unwrap().name, "kilocode");
    assert_eq!(get_provider_profile("kilo").unwrap().name, "kilocode");
    assert_eq!(
        get_provider_profile("kilo-gateway").unwrap().name,
        "kilocode"
    );
}

#[test]
fn kilocode_is_listed_once_by_canonical_name() {
    let _guard = KILOCODE_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "kilocode").count(), 1);
}
