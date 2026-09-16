//! Source-derived parity oracle for
//! `plugins/model-providers/alibaba-coding-plan/__init__.py` @ 5d59366.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle, plus
//! `TestAlibabaRegionalAndTokenPlanProfiles::test_alibaba_coding_plan_cn_registered`
//! in `tests/providers/test_provider_profiles.py` for the CN row.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static ALIBABA_CODING_PLAN_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn alibaba_coding_plan_profile_fields_and_aliases_match_upstream() {
    let _guard = ALIBABA_CODING_PLAN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("alibaba-coding-plan")
        .expect("Alibaba Coding Plan profile must be registered");

    assert_eq!(profile.name, "alibaba-coding-plan");
    assert_eq!(
        profile.aliases,
        ["alibaba_coding", "alibaba-coding", "dashscope-coding"]
    );
    assert_eq!(profile.display_name, "Alibaba Cloud (Coding Plan)");
    assert_eq!(
        profile.description,
        "Alibaba Cloud Coding Plan (Dedicated coding tier)"
    );
    assert_eq!(
        profile.signup_url,
        "https://help.aliyun.com/zh/model-studio/"
    );
    assert_eq!(
        profile.env_vars,
        [
            "ALIBABA_CODING_PLAN_API_KEY",
            "DASHSCOPE_API_KEY",
            "ALIBABA_CODING_PLAN_BASE_URL"
        ]
    );
    assert_eq!(
        profile.base_url,
        "https://coding-intl.dashscope.aliyuncs.com/v1"
    );
    assert_eq!(profile.get_hostname(), "coding-intl.dashscope.aliyuncs.com");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(
        get_provider_profile("alibaba_coding").unwrap().name,
        "alibaba-coding-plan"
    );
    assert_eq!(
        get_provider_profile("alibaba-coding").unwrap().name,
        "alibaba-coding-plan"
    );
    assert_eq!(
        get_provider_profile("dashscope-coding").unwrap().name,
        "alibaba-coding-plan"
    );
}

#[test]
fn alibaba_coding_plan_is_listed_once_by_canonical_name() {
    let _guard = ALIBABA_CODING_PLAN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names
            .iter()
            .filter(|name| *name == "alibaba-coding-plan")
            .count(),
        1
    );
}
#[test]
fn alibaba_coding_plan_cn_profile_fields_and_aliases_match_upstream() {
    // PARITY: `TestAlibabaRegionalAndTokenPlanProfiles::test_alibaba_coding_plan_cn_registered`
    // (`tests/providers/test_provider_profiles.py` @ 5d59366) plus the
    // declarative fields in `plugins/model-providers/alibaba-coding-plan/__init__.py`.
    let _guard = ALIBABA_CODING_PLAN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("alibaba-coding-plan-cn")
        .expect("Alibaba Coding Plan CN profile must be registered");

    assert_eq!(profile.name, "alibaba-coding-plan-cn");
    assert_eq!(
        profile.aliases,
        ["alibaba-coding-cn", "dashscope-coding-cn"]
    );
    assert_eq!(profile.display_name, "Alibaba Cloud (Coding Plan, China)");
    assert_eq!(
        profile.description,
        "Alibaba Cloud Coding Plan, mainland-China endpoint"
    );
    assert_eq!(
        profile.signup_url,
        "https://help.aliyun.com/zh/model-studio/"
    );
    assert_eq!(
        profile.env_vars,
        [
            "ALIBABA_CODING_PLAN_CN_API_KEY",
            "ALIBABA_CODING_PLAN_API_KEY",
            "DASHSCOPE_API_KEY",
            "ALIBABA_CODING_PLAN_CN_BASE_URL"
        ]
    );
    assert_eq!(profile.base_url, "https://coding.dashscope.aliyuncs.com/v1");
    assert_eq!(profile.get_hostname(), "coding.dashscope.aliyuncs.com");
    assert_eq!(profile.auth_type, "api_key");
    // The CN profile checks its own key first and keeps the shared vars as
    // ordered fallbacks so existing CN users keep working.
    assert_eq!(profile.env_vars[0], "ALIBABA_CODING_PLAN_CN_API_KEY");
    assert!(profile
        .env_vars
        .contains(&"ALIBABA_CODING_PLAN_API_KEY".to_string()));
    assert!(profile.env_vars.contains(&"DASHSCOPE_API_KEY".to_string()));
    assert_eq!(
        get_provider_profile("alibaba-coding-cn").unwrap().name,
        "alibaba-coding-plan-cn"
    );
    assert_eq!(
        get_provider_profile("dashscope-coding-cn").unwrap().name,
        "alibaba-coding-plan-cn"
    );
}
