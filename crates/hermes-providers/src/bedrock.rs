//! AWS Bedrock provider profile.
//!
//! PARITY: `plugins/model-providers/bedrock/__init__.py` @ 5d59366.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("bedrock");
    profile.aliases = vec![
        "aws".into(),
        "aws-bedrock".into(),
        "amazon-bedrock".into(),
        "amazon".into(),
    ];
    profile.api_mode = "bedrock_converse".into();
    profile.base_url = "https://bedrock-runtime.us-east-1.amazonaws.com".into();
    profile.auth_type = "aws_sdk".into();
    // PARITY: `supports_model_listing=False` — listing goes through the AWS
    // SDK, not a REST call (upstream replaced the `fetch_models` override
    // with this flag at 5d59366; the base gate returns None).
    profile.supports_model_listing = false;
    profile
}
