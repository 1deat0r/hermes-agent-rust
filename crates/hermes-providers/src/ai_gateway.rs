//! Vercel AI Gateway provider profile.
//!
//! PARITY: `plugins/model-providers/ai-gateway/__init__.py` @ b9aa928.
//! AI Gateway routes to multiple backends and receives Hermes attribution
//! headers plus the full reasoning configuration passthrough.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("ai-gateway");
    profile.aliases = vec![
        "vercel".into(),
        "vercel-ai-gateway".into(),
        "ai_gateway".into(),
        "aigateway".into(),
    ];
    profile.env_vars = vec!["AI_GATEWAY_API_KEY".into()];
    profile.base_url = "https://ai-gateway.vercel.sh/v1".into();
    profile.default_headers.insert(
        "HTTP-Referer".into(),
        "https://hermes-agent.nousresearch.com".into(),
    );
    profile
        .default_headers
        .insert("X-Title".into(), "Hermes Agent".into());
    profile.default_aux_model = "google/gemini-3-flash".into();
    // PARITY: VercelAIGatewayProfile.build_api_kwargs_extras routes the
    // reasoning configuration into extra_body.reasoning.
    profile.reasoning_passthrough = true;
    profile
}
