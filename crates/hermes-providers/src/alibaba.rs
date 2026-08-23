//! Alibaba Cloud DashScope provider profile.
//!
//! PARITY: `plugins/model-providers/alibaba/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("alibaba");
    profile.aliases = vec![
        "dashscope".into(),
        "alibaba-cloud".into(),
        "qwen-dashscope".into(),
    ];
    profile.env_vars = vec!["DASHSCOPE_API_KEY".into()];
    profile.base_url = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".into();
    profile
}
