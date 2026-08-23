//! Source-derived parity oracle for
//! `plugins/model-providers/openai-codex/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static OPENAI_CODEX_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn openai_codex_profile_fields_and_aliases_match_upstream() {
    let _guard = OPENAI_CODEX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile =
        get_provider_profile("openai-codex").expect("OpenAI Codex profile must be registered");

    assert_eq!(profile.name, "openai-codex");
    assert_eq!(profile.aliases, ["codex", "openai_codex"]);
    assert_eq!(profile.api_mode, "codex_responses");
    assert!(profile.env_vars.is_empty());
    assert_eq!(profile.base_url, "https://chatgpt.com/backend-api/codex");
    assert_eq!(profile.get_hostname(), "chatgpt.com");
    assert_eq!(profile.auth_type, "oauth_external");
    assert_eq!(get_provider_profile("codex").unwrap().name, "openai-codex");
    assert_eq!(
        get_provider_profile("openai_codex").unwrap().name,
        "openai-codex"
    );
}

#[test]
fn openai_codex_is_listed_once_by_canonical_name() {
    let _guard = OPENAI_CODEX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names.iter().filter(|name| *name == "openai-codex").count(),
        1
    );
}
