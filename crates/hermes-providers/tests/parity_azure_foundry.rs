//! Source-derived parity oracle for
//! `plugins/model-providers/azure-foundry/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static AZURE_FOUNDRY_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn azure_foundry_profile_fields_and_aliases_match_upstream() {
    let _guard = AZURE_FOUNDRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile =
        get_provider_profile("azure-foundry").expect("Azure Foundry profile must be registered");

    assert_eq!(profile.name, "azure-foundry");
    assert_eq!(profile.aliases, ["azure", "azure-ai-foundry", "azure-ai"]);
    assert_eq!(profile.display_name, "Azure Foundry");
    assert_eq!(
        profile.description,
        "Microsoft Foundry - OpenAI-compatible endpoint (user-supplied base URL)"
    );
    assert_eq!(profile.signup_url, "https://ai.azure.com/");
    assert_eq!(
        profile.env_vars,
        ["AZURE_FOUNDRY_API_KEY", "AZURE_FOUNDRY_BASE_URL"]
    );
    assert_eq!(profile.base_url, "");
    assert_eq!(profile.get_hostname(), "");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(get_provider_profile("azure").unwrap().name, "azure-foundry");
    assert_eq!(
        get_provider_profile("azure-ai-foundry").unwrap().name,
        "azure-foundry"
    );
    assert_eq!(
        get_provider_profile("azure-ai").unwrap().name,
        "azure-foundry"
    );
}

#[test]
fn azure_foundry_is_listed_once_by_canonical_name() {
    let _guard = AZURE_FOUNDRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names.iter().filter(|name| *name == "azure-foundry").count(),
        1
    );
}
