//! Source-derived parity oracle for
//! `plugins/model-providers/bedrock/__init__.py` @ b9aa928.
//!
//! The upstream profile uses a subclass override to disable REST model
//! discovery because Bedrock lists models through the AWS SDK. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static BEDROCK_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn bedrock_profile_fields_aliases_and_fetch_override_match_upstream() {
    let _guard = BEDROCK_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("bedrock").expect("Bedrock profile must be registered");

    assert_eq!(profile.name, "bedrock");
    assert_eq!(
        profile.aliases,
        ["aws", "aws-bedrock", "amazon-bedrock", "amazon"]
    );
    assert_eq!(profile.api_mode, "bedrock_converse");
    assert!(profile.env_vars.is_empty());
    assert_eq!(
        profile.base_url,
        "https://bedrock-runtime.us-east-1.amazonaws.com"
    );
    assert_eq!(
        profile.get_hostname(),
        "bedrock-runtime.us-east-1.amazonaws.com"
    );
    assert_eq!(profile.auth_type, "aws_sdk");
    assert_eq!(profile.fetch_models(None, None, 0.0), None);
    assert_eq!(get_provider_profile("aws").unwrap().name, "bedrock");
    assert_eq!(
        get_provider_profile("amazon-bedrock").unwrap().name,
        "bedrock"
    );
    assert_eq!(get_provider_profile("amazon").unwrap().name, "bedrock");
}

#[test]
fn bedrock_is_listed_once_by_canonical_name() {
    let _guard = BEDROCK_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "bedrock").count(), 1);
}
