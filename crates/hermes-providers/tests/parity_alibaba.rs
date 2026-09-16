//! Source-derived parity oracle for
//! `plugins/model-providers/alibaba/__init__.py` @ 5d59366.
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
        ["dashscope", "alibaba-cloud", "qwen-dashscope", "aliyun"]
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

/// PARITY @ 5d59366: CN, Token Plan, and Token Plan CN profiles (profile
/// names match models.dev catalog keys exactly).
#[test]
fn alibaba_sibling_profiles_match_upstream() {
    let _guard = ALIBABA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let cn = get_provider_profile("alibaba-cn").expect("alibaba-cn registered");
    assert_eq!(cn.aliases, ["dashscope-cn", "alibaba-cloud-cn"]);
    assert_eq!(cn.display_name, "Alibaba Cloud DashScope (China)");
    assert_eq!(cn.env_vars, ["DASHSCOPE_API_KEY", "DASHSCOPE_CN_BASE_URL"]);
    assert_eq!(cn.base_url, "https://dashscope.aliyuncs.com/compatible-mode/v1");
    assert_eq!(get_provider_profile("dashscope-cn").unwrap().name, "alibaba-cn");

    let tp = get_provider_profile("alibaba-token-plan").expect("token plan registered");
    assert_eq!(tp.aliases, ["dashscope-token-plan"]);
    assert_eq!(tp.display_name, "Alibaba Cloud (Token Plan)");
    assert_eq!(
        tp.env_vars,
        ["ALIBABA_TOKEN_PLAN_API_KEY", "ALIBABA_TOKEN_PLAN_BASE_URL"]
    );
    assert_eq!(
        tp.base_url,
        "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1"
    );

    let tpcn = get_provider_profile("alibaba-token-plan-cn").expect("token plan cn registered");
    assert_eq!(tpcn.aliases, ["dashscope-token-plan-cn"]);
    assert_eq!(
        tpcn.env_vars,
        [
            "ALIBABA_TOKEN_PLAN_CN_API_KEY",
            "ALIBABA_TOKEN_PLAN_API_KEY",
            "ALIBABA_TOKEN_PLAN_CN_BASE_URL"
        ]
    );
    assert_eq!(
        tpcn.base_url,
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1"
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
