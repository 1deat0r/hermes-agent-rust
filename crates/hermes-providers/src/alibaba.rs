//! Alibaba Cloud DashScope provider profiles (intl + CN + Token Plan).
//!
//! PARITY: `plugins/model-providers/alibaba/__init__.py` @ 5d59366.
//! Profile names match models.dev catalog keys exactly so model metadata
//! lines up and `model.provider: alibaba-cn` resolves at runtime.
use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba");
    profile.aliases = vec![
        "dashscope".into(),
        "alibaba-cloud".into(),
        "qwen-dashscope".into(),
        "aliyun".into(),
    ];
    profile.env_vars = vec!["DASHSCOPE_API_KEY".into()];
    profile.base_url = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".into();
    profile
}

pub(crate) fn cn_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba-cn");
    profile.aliases = vec!["dashscope-cn".into(), "alibaba-cloud-cn".into()];
    profile.display_name = "Alibaba Cloud DashScope (China)".into();
    profile.description = "Alibaba Cloud DashScope, mainland-China endpoint".into();
    profile.env_vars = vec!["DASHSCOPE_API_KEY".into(), "DASHSCOPE_CN_BASE_URL".into()];
    profile.base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1".into();
    profile
}

pub(crate) fn token_plan_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba-token-plan");
    profile.aliases = vec!["dashscope-token-plan".into()];
    profile.display_name = "Alibaba Cloud (Token Plan)".into();
    profile.description = "Alibaba Cloud Model Studio Token Plan (flat-token tier)".into();
    profile.signup_url = "https://help.aliyun.com/zh/model-studio/".into();
    profile.env_vars = vec![
        "ALIBABA_TOKEN_PLAN_API_KEY".into(),
        "ALIBABA_TOKEN_PLAN_BASE_URL".into(),
    ];
    profile.base_url =
        "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1".into();
    profile.auth_type = "api_key".into();
    profile
}

pub(crate) fn token_plan_cn_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba-token-plan-cn");
    profile.aliases = vec!["dashscope-token-plan-cn".into()];
    profile.display_name = "Alibaba Cloud (Token Plan, China)".into();
    profile.description = "Alibaba Cloud Model Studio Token Plan, mainland-China endpoint".into();
    profile.signup_url = "https://help.aliyun.com/zh/model-studio/".into();
    profile.env_vars = vec![
        "ALIBABA_TOKEN_PLAN_CN_API_KEY".into(),
        "ALIBABA_TOKEN_PLAN_API_KEY".into(),
        "ALIBABA_TOKEN_PLAN_CN_BASE_URL".into(),
    ];
    profile.base_url = "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into();
    profile.auth_type = "api_key".into();
    profile
}
