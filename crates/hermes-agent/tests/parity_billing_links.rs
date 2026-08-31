//! Parity tests for `agent/billing_links.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use hermes_agent::billing_links::{build_billing_block, is_nous_inference_route, BillingBlock};

#[test]
fn nous_route_gets_the_in_app_surface() {
    let block = build_billing_block("nous", "", "glm-4", "you hit a credit wall");
    assert_eq!(block.provider, "nous");
    assert_eq!(block.provider_label, "Nous Portal");
    assert!(block.is_nous, "routing bit set for Nous");
    assert_eq!(
        block.billing_url.as_deref(),
        Some("https://portal.nousresearch.com/billing")
    );
    assert_eq!(block.message, "you hit a credit wall");
}

#[test]
fn nous_route_also_detected_via_base_url_host() {
    // Detection is case-insensitive on the provider and host-based on the
    // URL even when the provider slug differs.
    assert!(is_nous_inference_route("NOUS", ""));
    assert!(is_nous_inference_route(
        "custom",
        "https://inference-api.nousresearch.com/v1"
    ));
    assert!(!is_nous_inference_route("openai", ""));
}

#[test]
fn known_slug_resolves_label_and_url() {
    for (slug, label, url) in [
        (
            "openai",
            "OpenAI",
            "https://platform.openai.com/settings/organization/billing",
        ),
        (
            "anthropic",
            "Anthropic",
            "https://console.anthropic.com/settings/billing",
        ),
        (
            "openrouter",
            "OpenRouter",
            "https://openrouter.ai/settings/credits",
        ),
        (
            "xai-oauth",
            "xAI",
            "https://console.x.ai/team/default/billing",
        ),
        (
            "deepseek",
            "DeepSeek",
            "https://platform.deepseek.com/top_up",
        ),
        ("groq", "Groq", "https://console.groq.com/settings/billing"),
        ("mistral", "Mistral", "https://console.mistral.ai/billing"),
        (
            "together",
            "Together AI",
            "https://api.together.ai/settings/billing",
        ),
        (
            "fireworks",
            "Fireworks AI",
            "https://fireworks.ai/account/billing",
        ),
        (
            "perplexity",
            "Perplexity",
            "https://www.perplexity.ai/settings/api",
        ),
        (
            "google",
            "Google AI",
            "https://aistudio.google.com/app/billing",
        ),
        (
            "gemini",
            "Google AI",
            "https://aistudio.google.com/app/billing",
        ),
        ("cohere", "Cohere", "https://dashboard.cohere.com/billing"),
        (
            "moonshot",
            "Moonshot AI",
            "https://platform.moonshot.ai/console/pay",
        ),
        (
            "nvidia",
            "NVIDIA",
            "https://build.nvidia.com/settings/billing",
        ),
    ] {
        let block = build_billing_block(slug, "", "m", "msg");
        assert_eq!(block.provider_label, label, "{slug}");
        assert_eq!(block.billing_url.as_deref(), Some(url), "{slug}");
        assert!(!block.is_nous, "{slug} is not Nous");
    }
}

#[test]
fn base_url_host_fallback_resolves_the_upstream() {
    // OpenAI-compatible generic slug with a recognizable upstream host.
    let block = build_billing_block("openai_compatible", "https://api.deepseek.com/v1", "m", "");
    assert_eq!(block.provider_label, "DeepSeek");
    assert_eq!(
        block.billing_url.as_deref(),
        Some("https://platform.deepseek.com/top_up")
    );
}

#[test]
fn unknown_provider_degrades_to_readable_label_without_invented_url() {
    let block = build_billing_block("some_new_cloud", "", "m", "");
    assert_eq!(block.provider, "some_new_cloud");
    assert_eq!(block.provider_label, "Some New Cloud");
    assert_eq!(block.billing_url, None);
    assert!(!block.is_nous);
}

#[test]
fn to_dict_carries_all_fields() {
    let block = BillingBlock {
        provider: "nous".to_string(),
        provider_label: "Nous Portal".to_string(),
        model: "glm-4".to_string(),
        billing_url: Some("https://portal/billing".to_string()),
        is_nous: true,
        message: "m".to_string(),
    };
    let dict = block.to_dict();
    assert_eq!(dict["provider"], "nous");
    assert_eq!(dict["provider_label"], "Nous Portal");
    assert_eq!(dict["model"], "glm-4");
    assert_eq!(dict["billing_url"], "https://portal/billing");
    assert_eq!(dict["is_nous"], true);
    assert_eq!(dict["message"], "m");
}
