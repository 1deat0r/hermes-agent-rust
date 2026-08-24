use hermes_agent::auxiliary_client::{
    auxiliary_max_tokens_param, is_model_incompatible_error, is_model_not_found_error,
    is_payment_error, is_rate_limit_error, normalize_aux_provider, AuxiliaryError,
};
use serde_json::json;

fn error(status_code: Option<u16>, type_name: &str, message: &str) -> AuxiliaryError {
    AuxiliaryError::new(status_code, type_name, message)
}

#[test]
fn normalize_aux_provider_aliases_and_special_forms_match_source() {
    for (input, expected) in [
        (None, "auto"),
        (Some(" GitHub-Models "), "copilot"),
        (Some("github-copilot-acp"), "copilot-acp"),
        (Some("glm"), "zai"),
        (Some("moonshot"), "kimi-coding"),
        (Some("moonshot-cn"), "kimi-coding-cn"),
        (Some("gmicloud"), "gmi"),
        (Some("actualcomputer"), "actual"),
        (Some("minimax_cn"), "minimax-cn"),
        (Some("claude-code"), "anthropic"),
        (Some("tencentmaas"), "tencent-tokenhub"),
        (Some("codex"), "openai-codex"),
        (Some("custom: z-ai"), "zai"),
        (Some("custom:"), "custom"),
        (Some("unknown-provider"), "unknown-provider"),
    ] {
        assert_eq!(normalize_aux_provider(input, None), expected, "{input:?}");
    }
    assert_eq!(
        normalize_aux_provider(Some("main"), Some("github-models")),
        "copilot"
    );
    assert_eq!(normalize_aux_provider(Some("main"), Some("auto")), "custom");
    assert_eq!(normalize_aux_provider(Some("main"), None), "custom");
}

#[test]
fn auxiliary_max_tokens_param_matches_url_key_and_model_precedence() {
    assert_eq!(
        auxiliary_max_tokens_param(4096, None, Some("https://api.openai.com/v1"), false, false),
        json!({"max_completion_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(
            4096,
            None,
            Some("https://api.githubcopilot.com"),
            false,
            false
        ),
        json!({"max_completion_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(
            4096,
            None,
            Some("https://enterprise.githubcopilot.com/v1"),
            false,
            false
        ),
        json!({"max_completion_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(4096, None, Some("https://api.openai.com/v1"), true, false),
        json!({"max_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(4096, None, Some("https://api.openai.com/v1"), false, true),
        json!({"max_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(
            4096,
            Some("custom/gpt-5.4"),
            Some("https://gateway.example/v1"),
            false,
            false
        ),
        json!({"max_completion_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(
            4096,
            Some("gpt-4o-mini"),
            Some("https://gateway.example/v1"),
            false,
            false
        ),
        json!({"max_completion_tokens": 4096}),
    );
    assert_eq!(
        auxiliary_max_tokens_param(
            4096,
            Some("glm-5.2"),
            Some("https://gateway.example/v1"),
            false,
            false
        ),
        json!({"max_tokens": 4096}),
    );
}

#[test]
fn payment_error_classification_matches_source_keywords_and_statuses() {
    assert!(is_payment_error(&error(
        Some(402),
        "Exception",
        "Payment Required"
    )));
    assert!(is_payment_error(&error(
        Some(403),
        "Exception",
        "this model requires a subscription, upgrade for access",
    )));
    assert!(is_payment_error(&error(
        Some(429),
        "Exception",
        "quota exceeded"
    )));
    assert!(is_payment_error(&error(
        None,
        "Exception",
        "resource exhausted"
    )));
    assert!(!is_payment_error(&error(
        Some(404),
        "Exception",
        "Not Found"
    )));
    assert!(!is_payment_error(&error(
        Some(500),
        "Exception",
        "billing backend down"
    )));
}

#[test]
fn rate_limit_classification_distinguishes_billing_and_sdk_type_name() {
    assert!(is_rate_limit_error(&error(
        Some(429),
        "Exception",
        "Rate limit exceeded, try again in 2 seconds",
    )));
    assert!(is_rate_limit_error(&error(
        None,
        "RateLimitError",
        "provider response"
    )));
    assert!(is_rate_limit_error(&error(
        Some(429),
        "Exception",
        "Too many requests"
    )));
    assert!(!is_rate_limit_error(&error(
        Some(429),
        "Exception",
        "insufficient funds"
    )));
    assert!(!is_rate_limit_error(&error(
        Some(400),
        "Exception",
        "rate limit"
    )));
}

#[test]
fn model_not_found_and_incompatible_classifiers_are_disjoint() {
    let stale = error(
        Some(404),
        "Exception",
        "Model 'gpt-5.4-mini' not found. The requested model does not exist in our configuration or OpenRouter catalog.",
    );
    assert!(is_model_not_found_error(&stale));
    assert!(!is_model_incompatible_error(&stale));

    let invalid = error(
        Some(400),
        "Exception",
        "openrouter/foo is not a valid model ID",
    );
    assert!(is_model_not_found_error(&invalid));
    assert!(!is_model_incompatible_error(&invalid));

    let incompatible = error(
        Some(400),
        "Exception",
        "The glm-5.2 model is not supported when using Codex with a ChatGPT account.",
    );
    assert!(is_model_incompatible_error(&incompatible));
    assert!(!is_model_not_found_error(&incompatible));

    let billing = error(
        Some(400),
        "Exception",
        "insufficient credits: model is not supported on free tier",
    );
    assert!(!is_model_incompatible_error(&billing));
    assert!(!is_model_not_found_error(&billing));
}
