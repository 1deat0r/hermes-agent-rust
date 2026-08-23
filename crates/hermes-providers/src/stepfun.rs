//! StepFun provider profile.
//!
//! PARITY: `plugins/model-providers/stepfun/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("stepfun");
    profile.aliases = vec!["step".into(), "stepfun-coding-plan".into()];
    profile.default_aux_model = "step-3.5-flash".into();
    profile.env_vars = vec!["STEPFUN_API_KEY".into()];
    profile.base_url = "https://api.stepfun.ai/step_plan/v1".into();
    profile
}
