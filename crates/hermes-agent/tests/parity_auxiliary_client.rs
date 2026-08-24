use hermes_agent::auxiliary_client::{
    auxiliary_max_tokens_param, is_anthropic_compatible_host, is_model_incompatible_error,
    is_model_not_found_error, is_payment_error, is_rate_limit_error, normalize_aux_provider,
    pool_runtime_api_key, pool_runtime_base_url, resolve_aux_task_provider_model,
    to_openai_base_url, AuxiliaryError, AuxiliaryPoolEntry, AuxiliaryTaskConfig,
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

const KNOWN_PROVIDER_IDS: &[&str] = &[
    "anthropic",
    "minimax-oauth",
    "nous",
    "openai-codex",
    "qwen-oauth",
    "xai-oauth",
];

#[test]
fn resolve_task_provider_preserves_known_provider_with_explicit_endpoint() {
    for provider in KNOWN_PROVIDER_IDS {
        let resolved = resolve_aux_task_provider_model(
            None,
            Some("vision"),
            Some(provider),
            Some("test-model"),
            Some("https://provider.example/v1"),
            Some("resolved-token"),
            None,
            KNOWN_PROVIDER_IDS,
        );

        assert_eq!(resolved.provider, *provider);
        assert_eq!(resolved.model.as_deref(), Some("test-model"));
        assert_eq!(
            resolved.base_url.as_deref(),
            Some("https://provider.example/v1")
        );
        assert_eq!(resolved.api_key.as_deref(), Some("resolved-token"));
        assert_eq!(resolved.api_mode, None);
    }
}

#[test]
fn resolve_task_provider_adopts_matching_configured_endpoint_and_key() {
    let config = AuxiliaryTaskConfig {
        provider: Some("custom".into()),
        model: Some("meta/llama-3.2-11b-vision-instruct".into()),
        base_url: Some("https://integrate.api.nvidia.com/v1".into()),
        api_key: Some("nvapi-secret".into()),
        api_mode: None,
    };

    let resolved = resolve_aux_task_provider_model(
        Some(&config),
        Some("vision"),
        Some("custom"),
        Some("meta/llama-3.2-11b-vision-instruct"),
        None,
        None,
        None,
        &[],
    );

    assert_eq!(resolved.provider, "custom");
    assert_eq!(
        resolved.model.as_deref(),
        Some("meta/llama-3.2-11b-vision-instruct")
    );
    assert_eq!(
        resolved.base_url.as_deref(),
        Some("https://integrate.api.nvidia.com/v1")
    );
    assert_eq!(resolved.api_key.as_deref(), Some("nvapi-secret"));
    assert_eq!(resolved.api_mode, None);
}

#[test]
fn resolve_task_provider_unwraps_explicit_moa_and_drops_virtual_credentials() {
    let resolved = resolve_aux_task_provider_model(
        None,
        Some("title_generation"),
        Some("moa"),
        Some("opus-gpt"),
        Some("moa://local"),
        Some("moa-virtual-provider"),
        Some(("openrouter", "anthropic/claude-opus-4.8")),
        &[],
    );

    assert_eq!(resolved.provider, "openrouter");
    assert_eq!(resolved.model.as_deref(), Some("anthropic/claude-opus-4.8"));
    assert_eq!(resolved.base_url, None);
    assert_eq!(resolved.api_key, None);
}

#[test]
fn resolve_task_provider_unwraps_configured_moa() {
    let config = AuxiliaryTaskConfig {
        provider: Some("moa".into()),
        model: Some("opus-gpt".into()),
        ..AuxiliaryTaskConfig::default()
    };

    let resolved = resolve_aux_task_provider_model(
        Some(&config),
        Some("title_generation"),
        None,
        None,
        None,
        None,
        Some(("anthropic", "claude-opus-4.8")),
        &[],
    );

    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.model.as_deref(), Some("claude-opus-4.8"));
    assert_eq!(resolved.base_url, None);
    assert_eq!(resolved.api_key, None);
}

#[test]
fn resolve_task_provider_keeps_literal_moa_when_preset_is_unresolved() {
    let resolved = resolve_aux_task_provider_model(
        None,
        Some("title_generation"),
        Some("moa"),
        Some("gone-preset"),
        None,
        None,
        None,
        &[],
    );

    assert_eq!(resolved.provider, "moa");
    assert_eq!(resolved.model.as_deref(), Some("gone-preset"));
}

#[test]
fn resolve_task_provider_normalizes_explicit_auto_model_to_none() {
    let resolved = resolve_aux_task_provider_model(
        None,
        None,
        Some("anthropic"),
        Some("auto"),
        None,
        None,
        None,
        &[],
    );

    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.model, None);
}

#[test]
fn resolve_task_provider_expands_direct_openai_alias() {
    let resolved = resolve_aux_task_provider_model(
        None,
        None,
        Some("openai"),
        Some("gpt-5.4"),
        None,
        None,
        None,
        &[],
    );

    assert_eq!(resolved.provider, "custom");
    assert_eq!(
        resolved.base_url.as_deref(),
        Some("https://api.openai.com/v1")
    );
    assert_eq!(resolved.model.as_deref(), Some("gpt-5.4"));
}

#[test]
fn pool_runtime_api_key_prefers_projected_key_then_access_token() {
    let entry = AuxiliaryPoolEntry {
        runtime_api_key: Some("  runtime-key  ".into()),
        access_token: Some("access-token".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(pool_runtime_api_key(Some(&entry)), "runtime-key");

    let fallback_entry = AuxiliaryPoolEntry {
        access_token: Some("  access-token  ".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(pool_runtime_api_key(Some(&fallback_entry)), "access-token");
    assert_eq!(pool_runtime_api_key(None), "");
}

#[test]
fn pool_runtime_base_url_follows_source_precedence_and_normalization() {
    let runtime = AuxiliaryPoolEntry {
        runtime_base_url: Some(" https://runtime.example/v1/// ".into()),
        inference_base_url: Some("https://inference.example/v1".into()),
        base_url: Some("https://base.example/v1".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        pool_runtime_base_url(Some(&runtime), Some("https://fallback.example/v1"), None),
        "https://runtime.example/v1"
    );

    let inference = AuxiliaryPoolEntry {
        inference_base_url: Some(" https://inference.example/v1/ ".into()),
        base_url: Some("https://base.example/v1".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        pool_runtime_base_url(Some(&inference), Some("https://fallback.example/v1"), None),
        "https://inference.example/v1"
    );

    let base = AuxiliaryPoolEntry {
        base_url: Some(" https://base.example/v1/// ".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        pool_runtime_base_url(Some(&base), Some("https://fallback.example/v1"), None),
        "https://base.example/v1"
    );
    assert_eq!(
        pool_runtime_base_url(None, Some(" https://fallback.example/v1/// "), None),
        "https://fallback.example/v1"
    );
}

#[test]
fn pool_runtime_base_url_applies_only_nous_inference_override() {
    let nous = AuxiliaryPoolEntry {
        provider: Some("nous".into()),
        runtime_base_url: Some("https://runtime.nous.example/v1".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        pool_runtime_base_url(
            Some(&nous),
            Some("https://fallback.example/v1"),
            Some(" https://override.nous.example/v1/// "),
        ),
        "https://override.nous.example/v1"
    );

    let other = AuxiliaryPoolEntry {
        provider: Some("openrouter".into()),
        runtime_base_url: Some("https://runtime.example/v1".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        pool_runtime_base_url(
            Some(&other),
            Some("https://fallback.example/v1"),
            Some("https://override.nous.example/v1"),
        ),
        "https://runtime.example/v1"
    );
}

#[test]
fn openai_base_url_normalization_matches_provider_wire_surfaces() {
    for (input, expected) in [
        (
            Some(" https://api.minimax.io/anthropic/// "),
            "https://api.minimax.io/v1",
        ),
        (
            Some("https://open.bigmodel.cn/api/anthropic"),
            "https://open.bigmodel.cn/api/paas/v4",
        ),
        (
            Some("https://api.kimi.com/coding/"),
            "https://api.kimi.com/coding/v1",
        ),
        (
            Some("https://proxy.example/anthropic"),
            "https://proxy.example/v1",
        ),
        (
            Some("https://provider.example/v1///"),
            "https://provider.example/v1",
        ),
        (None, ""),
    ] {
        assert_eq!(to_openai_base_url(input), expected, "{input:?}");
    }
}

#[test]
fn anthropic_compatible_host_guard_is_exact_and_fail_closed() {
    for url in [
        "https://api.anthropic.com",
        "https://API.ANTHROPIC.COM/v1",
        "https://api.anthropic.com./v1",
        "//api.anthropic.com/v1",
    ] {
        assert!(is_anthropic_compatible_host(url), "{url}");
    }
    for url in [
        "",
        "not a url",
        "https://openrouter.ai/api/v1",
        "https://api.anthropic.com.evil.example/v1",
        "https://proxy.api.anthropic.com/v1",
    ] {
        assert!(!is_anthropic_compatible_host(url), "{url}");
    }
}
