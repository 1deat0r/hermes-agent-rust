//! Source-derived parity oracle for
//! `plugins/model-providers/copilot-acp/__init__.py` @ b9aa928.
//!
//! No dedicated upstream test module exists for this profile, so the pinned
//! source is the oracle. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static COPILOT_ACP_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn copilot_acp_profile_fields_aliases_and_external_routing_match_upstream() {
    let _guard = COPILOT_ACP_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile =
        get_provider_profile("copilot-acp").expect("GitHub Copilot ACP profile must be registered");

    assert_eq!(profile.name, "copilot-acp");
    assert_eq!(profile.aliases, ["github-copilot-acp", "copilot-acp-agent"]);
    assert_eq!(profile.api_mode, "chat_completions");
    assert!(profile.env_vars.is_empty());
    assert_eq!(profile.base_url, "acp://copilot");
    assert_eq!(profile.auth_type, "external_process");
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert!(profile.default_headers.is_empty());
    assert!(profile.fallback_models.is_empty());
    assert!(profile.default_aux_model.is_empty());
}

#[test]
fn copilot_acp_delegates_model_listing_and_is_listed_once() {
    let _guard = COPILOT_ACP_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("copilot-acp").unwrap();

    // PARITY: CopilotACPProfile.fetch_models() always returns None because
    // model listing is handled by the external ACP subprocess.
    assert_eq!(profile.fetch_models(None, None, 8.0), None);
    assert!(profile.models_fetch_disabled);
    assert_eq!(
        get_provider_profile("github-copilot-acp").unwrap().name,
        "copilot-acp"
    );
    assert_eq!(
        get_provider_profile("copilot-acp-agent").unwrap().name,
        "copilot-acp"
    );

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names.iter().filter(|name| *name == "copilot-acp").count(),
        1
    );
}
