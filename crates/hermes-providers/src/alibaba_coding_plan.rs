//! Alibaba Cloud Coding Plan provider profiles (intl + CN).
//!
//! PARITY: `plugins/model-providers/alibaba-coding-plan/__init__.py` @ 5d59366.
use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba-coding-plan");
    profile.aliases = vec![
        "alibaba_coding".into(),
        "alibaba-coding".into(),
        "dashscope-coding".into(),
    ];
    profile.display_name = "Alibaba Cloud (Coding Plan)".into();
    profile.description = "Alibaba Cloud Coding Plan (Dedicated coding tier)".into();
    profile.signup_url = "https://help.aliyun.com/zh/model-studio/".into();
    profile.env_vars = vec![
        "ALIBABA_CODING_PLAN_API_KEY".into(),
        "DASHSCOPE_API_KEY".into(),
        "ALIBABA_CODING_PLAN_BASE_URL".into(),
    ];
    profile.base_url = "https://coding-intl.dashscope.aliyuncs.com/v1".into();
    profile.auth_type = "api_key".into();
    profile
}

pub(crate) fn cn_profile() -> ProviderProfile {
    // PARITY: `alibaba_coding_plan_cn` — the mainland-China endpoint. Its own
    // key comes first with the shared vars as ordered fallbacks so existing
    // CN users configured with the shared key keep working.
    let mut profile = ProviderProfile::new("alibaba-coding-plan-cn");
    profile.aliases = vec!["alibaba-coding-cn".into(), "dashscope-coding-cn".into()];
    profile.display_name = "Alibaba Cloud (Coding Plan, China)".into();
    profile.description = "Alibaba Cloud Coding Plan, mainland-China endpoint".into();
    profile.signup_url = "https://help.aliyun.com/zh/model-studio/".into();
    profile.env_vars = vec![
        "ALIBABA_CODING_PLAN_CN_API_KEY".into(),
        "ALIBABA_CODING_PLAN_API_KEY".into(),
        "DASHSCOPE_API_KEY".into(),
        "ALIBABA_CODING_PLAN_CN_BASE_URL".into(),
    ];
    profile.base_url = "https://coding.dashscope.aliyuncs.com/v1".into();
    profile.auth_type = "api_key".into();
    profile
}
