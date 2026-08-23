//! AWS Bedrock provider profile.
//!
//! PARITY: `plugins/model-providers/bedrock/__init__.py` @ b9aa928.

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
    // PARITY: BedrockProfile.fetch_models() returns None because the AWS SDK
    // is the model-discovery transport, not a REST /v1/models endpoint.
    profile.models_fetch_disabled = true;
    profile
}
