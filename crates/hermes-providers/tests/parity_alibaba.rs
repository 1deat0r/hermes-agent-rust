//! Source-derived parity oracle for
//! `plugins/model-providers/alibaba/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated test file; its declarative profile
//! fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static ALIBABA_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn alibaba_profile_fields_and_aliases_match_upstream() {
    let _guard = ALIBABA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("alibaba").expect("Alibaba profile must be registered");

    assert_eq!(profile.name, "alibaba");
    assert_eq!(
        profile.aliases,
        ["dashscope", "alibaba-cloud", "qwen-dashscope"]
    );
    assert_eq!(profile.env_vars, ["DASHSCOPE_API_KEY"]);
    assert_eq!(
        profile.base_url,
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1"
    );
    assert_eq!(profile.get_hostname(), "dashscope-intl.aliyuncs.com");
    assert_eq!(get_provider_profile("dashscope").unwrap().name, "alibaba");
    assert_eq!(
        get_provider_profile("alibaba-cloud").unwrap().name,
        "alibaba"
    );
    assert_eq!(
        get_provider_profile("qwen-dashscope").unwrap().name,
        "alibaba"
    );
}

#[test]
fn alibaba_is_listed_once_by_canonical_name() {
    let _guard = ALIBABA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "alibaba").count(), 1);
}
