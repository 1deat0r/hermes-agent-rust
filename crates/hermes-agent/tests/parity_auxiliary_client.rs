use hermes_agent::auxiliary_client::{
    apply_user_default_headers, auxiliary_default_headers, auxiliary_http_client_config,
    auxiliary_max_tokens_param, auxiliary_proxy_for_base_url, auxiliary_proxy_from_env,
    auxiliary_tls_verify_resolution, build_auxiliary_http_client, build_nvidia_nim_headers,
    build_or_headers, codex_cloudflare_headers, compression_threshold_for_model,
    copilot_request_headers, fixed_temperature_for_model, get_auxiliary_extra_body,
    is_anthropic_compatible_host, is_arcee_trinity_thinking, is_codex_gpt54_or_gpt55,
    is_codex_spark, is_kimi_model, is_model_incompatible_error, is_model_not_found_error,
    is_payment_error, is_rate_limit_error, normalize_aux_provider, nous_extra_body,
    nous_portal_fallback_extra, openai_client_config, openai_client_config_with_transport,
    openrouter_cache_headers, pool_runtime_api_key, pool_runtime_base_url, read_codex_access_token,
    resolve_aux_task_provider_model, resolve_auxiliary_tls_verify,
    resolve_pool_first_runtime_credentials, resolve_provider_default_headers,
    select_auxiliary_pool_entry, to_openai_base_url, AuxiliaryError, AuxiliaryHttpClient,
    AuxiliaryHttpClientConfig, AuxiliaryPoolEntry, AuxiliaryRuntimeCredentials,
    AuxiliarySslVerifySetting, AuxiliaryTaskConfig, AuxiliaryTemperaturePolicy, AuxiliaryTlsVerify,
};
use parking_lot::Mutex;
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::OsString;

const AUXILIARY_ENV_KEYS: &[&str] = &[
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
    "HERMES_CA_BUNDLE",
    "SSL_CERT_FILE",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
];

static AUXILIARY_ENV_MUTEX: Mutex<()> = Mutex::new(());

struct EnvironmentSnapshot {
    values: Vec<(&'static str, Option<OsString>)>,
}

impl EnvironmentSnapshot {
    fn new() -> Self {
        Self {
            values: AUXILIARY_ENV_KEYS
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect(),
        }
    }
}

impl Drop for EnvironmentSnapshot {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn clear_auxiliary_env() {
    for key in AUXILIARY_ENV_KEYS {
        unsafe { std::env::remove_var(key) };
    }
}

fn error(status_code: Option<u16>, type_name: &str, message: &str) -> AuxiliaryError {
    AuxiliaryError::new(status_code, type_name, message)
}
// Tier: unit — mirrors agent/auxiliary_client.py lines 560-697 and
// tests/agent/test_arcee_trinity_overrides.py,
// tests/hermes_cli/test_gpt56_registration.py, and
// tests/agent/test_auxiliary_client.py lines 2159-2212.
#[test]
fn auxiliary_model_policy_normalizes_kimi_and_arcee_models() {
    assert!(is_kimi_model(Some("  provider/KIMI-k2.5  ")));
    assert!(is_kimi_model(Some("kimi")));
    assert!(!is_kimi_model(Some("moonshot-v1")));
    assert!(!is_kimi_model(Some("kimiish")));

    for model in [
        Some("trinity-large-thinking"),
        Some("arcee-ai/trinity-large-thinking"),
        Some(" Arcee-AI/Trinity-Large-Thinking "),
    ] {
        assert!(is_arcee_trinity_thinking(model));
    }
    for model in [
        Some("trinity-large-preview"),
        Some("trinity-large-thinking-pro"),
        Some("arcee-ai/trinity-large-thinking-mini"),
    ] {
        assert!(!is_arcee_trinity_thinking(model));
    }
}

#[test]
fn auxiliary_temperature_policy_preserves_source_directives() {
    assert_eq!(
        fixed_temperature_for_model(Some("moonshot-v1"), None),
        AuxiliaryTemperaturePolicy::Default
    );
    assert_eq!(
        fixed_temperature_for_model(Some("openrouter/KIMI-k2.5"), None),
        AuxiliaryTemperaturePolicy::Omit
    );
    assert_eq!(
        fixed_temperature_for_model(Some("arcee-ai/trinity-large-thinking"), None),
        AuxiliaryTemperaturePolicy::Fixed(0.5)
    );
    assert_eq!(
        fixed_temperature_for_model(Some("trinity-large-thinking-pro"), None),
        AuxiliaryTemperaturePolicy::Default
    );
    assert_eq!(
        fixed_temperature_for_model(Some("gpt-5.6"), None),
        AuxiliaryTemperaturePolicy::Default
    );
}

#[test]
fn auxiliary_codex_model_predicates_require_exact_route_and_suffix_boundaries() {
    for family in ["gpt-5.4", "gpt-5.5", "gpt-5.6"] {
        for suffix in ["", "-pro", ".sol"] {
            let model = format!(" openai/{family}{suffix} ");
            assert!(is_codex_gpt54_or_gpt55(
                Some(model.as_str()),
                Some(" OPENAI-CODEX ")
            ));
        }
    }
    for model in [
        "gpt-5.45", "gpt-5.50", "gpt-5.55", "gpt-5.40", "gpt-5.60", "gpt-5",
    ] {
        assert!(!is_codex_gpt54_or_gpt55(Some(model), Some("openai-codex")));
    }
    for provider in ["openai", "openrouter", "github-copilot", " OPENAI "] {
        assert!(!is_codex_gpt54_or_gpt55(
            Some("openai/gpt-5.6-sol"),
            Some(provider)
        ));
    }
}

#[test]
fn auxiliary_spark_predicate_is_exact_and_codex_route_only() {
    assert!(is_codex_spark(
        Some(" openai/GPT-5.3-CODEX-SPARK "),
        Some(" OPENAI-CODEX ")
    ));
    for model in [
        "gpt-5.5",
        "gpt-5.3-codex",
        "gpt-5.3",
        "gpt-5.3-codex-spark-mini",
    ] {
        assert!(!is_codex_spark(Some(model), Some("openai-codex")));
    }
    assert!(!is_codex_spark(
        Some("gpt-5.3-codex-spark"),
        Some("openrouter")
    ));
}

#[test]
fn auxiliary_compression_threshold_precedence_and_flag_gating_match_source() {
    assert_eq!(
        compression_threshold_for_model(
            Some("arcee-ai/trinity-large-thinking"),
            Some("openai-codex"),
            true
        ),
        Some(0.75)
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.5"), Some("openai-codex"), true),
        Some(0.85)
    );
    assert_eq!(
        compression_threshold_for_model(
            Some("trinity-large-thinking"),
            Some("openai-codex"),
            false
        ),
        Some(0.75)
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.4-pro"), Some("openai-codex"), true),
        Some(0.85)
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.5"), Some("openai-codex"), false),
        None
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.6-luna"), Some("openai-codex"), true),
        Some(0.85)
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.6-sol"), Some("openai"), true),
        None
    );
    assert_eq!(
        compression_threshold_for_model(Some("openai/gpt-5.6-sol"), Some("openrouter"), true),
        None
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.6-luna"), Some("openrouter"), true),
        None
    );
    assert_eq!(
        compression_threshold_for_model(Some("gpt-5.3-codex-spark"), Some("openai-codex"), false),
        Some(0.70)
    );
    assert_eq!(
        compression_threshold_for_model(
            Some("gpt-5.3-codex-spark-mini"),
            Some("openai-codex"),
            true
        ),
        None
    );
    assert_eq!(
        compression_threshold_for_model(Some("claude-sonnet-4.6"), None, true),
        None
    );
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

#[test]
fn openai_client_config_disables_sdk_retries_by_default() {
    let config = openai_client_config("api-key", "https://provider.example/v1", None);
    assert_eq!(config.api_key, "api-key");
    assert_eq!(config.base_url, "https://provider.example/v1");
    assert_eq!(config.max_retries, 0);
}

#[test]
fn openai_client_config_preserves_explicit_retry_override() {
    let config = openai_client_config("api-key", "https://provider.example/v1", Some(5));
    assert_eq!(config.max_retries, 5);
}

// Tier: unit — mirrors agent/process_bootstrap.py build_keepalive_http_client.
#[test]
fn auxiliary_http_client_config_matches_keepalive_pool_and_timeout_contract() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let config = auxiliary_http_client_config(
        Some("https://provider.example/v1"),
        false,
        AuxiliaryTlsVerify::CaBundle("/tmp/provider-ca.pem".into()),
    );

    assert_eq!(
        config,
        AuxiliaryHttpClientConfig {
            async_mode: false,
            proxy: None,
            verify: AuxiliaryTlsVerify::CaBundle("/tmp/provider-ca.pem".into()),
            max_keepalive_connections: 20,
            max_connections: 100,
            keepalive_expiry: std::time::Duration::from_secs(20),
            connect_timeout: std::time::Duration::from_secs(15),
            read_timeout: None,
            write_timeout: std::time::Duration::from_secs(15),
            pool_timeout: std::time::Duration::from_secs(10),
            plain_scheme_mounts: true,
        }
    );
}

// Tier: unit — mirrors agent/process_bootstrap.py proxy/mount construction.
#[test]
fn auxiliary_http_client_config_uses_proxy_and_async_transport_precedence() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();
    unsafe { std::env::set_var("HTTPS_PROXY", "http://proxy.example:8080") };

    let config = auxiliary_http_client_config(
        Some("https://provider.example/v1"),
        true,
        AuxiliaryTlsVerify::Disabled,
    );

    assert!(config.async_mode);
    assert_eq!(config.proxy.as_deref(), Some("http://proxy.example:8080"));
    assert!(!config.plain_scheme_mounts);
    assert_eq!(config.verify, AuxiliaryTlsVerify::Disabled);
}

// Tier: unit — mirrors agent/auxiliary_client.py _create_openai_client kwargs merge.
#[test]
fn openai_client_config_with_transport_injects_default_and_preserves_explicit_client() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let defaulted = openai_client_config_with_transport(
        "api-key",
        "https://provider.example/v1",
        None,
        false,
        AuxiliaryTlsVerify::Default,
        None,
    );
    assert_eq!(defaulted.max_retries, 0);
    assert!(defaulted.http_client.is_some());

    let explicit = AuxiliaryHttpClientConfig {
        async_mode: true,
        proxy: Some("http://explicit.example:8080".into()),
        verify: AuxiliaryTlsVerify::Disabled,
        max_keepalive_connections: 1,
        max_connections: 2,
        keepalive_expiry: std::time::Duration::from_secs(3),
        connect_timeout: std::time::Duration::from_secs(4),
        read_timeout: Some(std::time::Duration::from_secs(5)),
        write_timeout: std::time::Duration::from_secs(6),
        pool_timeout: std::time::Duration::from_secs(7),
        plain_scheme_mounts: false,
    };
    let overridden = openai_client_config_with_transport(
        "api-key",
        "https://provider.example/v1",
        Some(4),
        false,
        AuxiliaryTlsVerify::Default,
        Some(explicit.clone()),
    );
    assert_eq!(overridden.max_retries, 4);
    assert_eq!(overridden.http_client, Some(explicit));
}

// Tier: unit — mirrors agent/process_bootstrap.py build_keepalive_http_client.
#[test]
fn build_auxiliary_http_client_constructs_sync_and_async_variants() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let sync = auxiliary_http_client_config(
        Some("https://provider.example/v1"),
        false,
        AuxiliaryTlsVerify::Default,
    );
    assert!(matches!(
        build_auxiliary_http_client(&sync),
        Some(AuxiliaryHttpClient::Blocking(_))
    ));

    let async_config = auxiliary_http_client_config(
        Some("https://provider.example/v1"),
        true,
        AuxiliaryTlsVerify::Disabled,
    );
    assert!(matches!(
        build_auxiliary_http_client(&async_config),
        Some(AuxiliaryHttpClient::Async(_))
    ));
}

// Tier: unit — mirrors source proxy forwarding into the constructed client.
#[test]
fn build_auxiliary_http_client_accepts_explicit_proxy() {
    let config = AuxiliaryHttpClientConfig {
        proxy: Some("http://proxy.example:8080".into()),
        plain_scheme_mounts: false,
        ..auxiliary_http_client_config(
            Some("https://provider.example/v1"),
            false,
            AuxiliaryTlsVerify::Default,
        )
    };

    assert!(matches!(
        build_auxiliary_http_client(&config),
        Some(AuxiliaryHttpClient::Blocking(_))
    ));
}

// Tier: unit — mirrors source builder's broad fail-open exception path.
#[test]
fn build_auxiliary_http_client_fails_open_for_unusable_ca_bundle() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let config = auxiliary_http_client_config(
        Some("https://provider.example/v1"),
        false,
        AuxiliaryTlsVerify::CaBundle("/definitely/missing/hermes-agent-rust-ca-bundle.pem".into()),
    );
    assert!(build_auxiliary_http_client(&config).is_none());
}

// Tier: unit — mirrors source's insecure verify forwarding to the client.
#[test]
fn build_auxiliary_http_client_accepts_explicit_insecure_tls_mode() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let config = AuxiliaryHttpClientConfig {
        verify: AuxiliaryTlsVerify::Disabled,
        ..auxiliary_http_client_config(
            Some("https://provider.example/v1"),
            false,
            AuxiliaryTlsVerify::Default,
        )
    };
    assert!(build_auxiliary_http_client(&config).is_some());
}

// Tier: unit — mirrors agent/auxiliary_client.py _select_pool_entry.
#[test]
fn auxiliary_pool_selection_preserves_fail_open_presence_states() {
    let entry = AuxiliaryPoolEntry {
        runtime_api_key: Some("pool-key".into()),
        ..AuxiliaryPoolEntry::default()
    };

    assert_eq!(
        select_auxiliary_pool_entry(false, true, Some(&entry)),
        (false, None)
    );
    assert_eq!(
        select_auxiliary_pool_entry(true, false, Some(&entry)),
        (false, None)
    );
    assert_eq!(select_auxiliary_pool_entry(true, true, None), (true, None));
    assert_eq!(
        select_auxiliary_pool_entry(true, true, Some(&entry)),
        (true, Some(&entry))
    );
}

// Tier: unit — mirrors Nous/xAI auxiliary pool-first runtime resolution.
#[test]
fn pool_first_runtime_credentials_prefer_valid_pool_then_legacy_fallback() {
    let pool_entry = AuxiliaryPoolEntry {
        runtime_api_key: Some("  pool-key  ".into()),
        runtime_base_url: Some(" https://pool.example/v1/// ".into()),
        ..AuxiliaryPoolEntry::default()
    };
    let legacy = Some((" legacy-key ", " https://legacy.example/v1/// "));

    assert_eq!(
        resolve_pool_first_runtime_credentials(true, Some(&pool_entry), None, None, legacy,),
        Some(AuxiliaryRuntimeCredentials {
            api_key: "pool-key".into(),
            base_url: "https://pool.example/v1".into(),
        })
    );

    let invalid_pool = AuxiliaryPoolEntry {
        runtime_api_key: Some(" ".into()),
        runtime_base_url: Some("https://pool.example/v1".into()),
        ..AuxiliaryPoolEntry::default()
    };
    assert_eq!(
        resolve_pool_first_runtime_credentials(true, Some(&invalid_pool), None, None, legacy),
        Some(AuxiliaryRuntimeCredentials {
            api_key: "legacy-key".into(),
            base_url: "https://legacy.example/v1".into(),
        })
    );
    assert_eq!(
        resolve_pool_first_runtime_credentials(false, None, None, None, legacy),
        Some(AuxiliaryRuntimeCredentials {
            api_key: "legacy-key".into(),
            base_url: "https://legacy.example/v1".into(),
        })
    );
}

// Tier: unit — mirrors tests/run_agent/test_create_openai_client_proxy_env.py.
#[test]
fn auxiliary_proxy_from_env_prefers_https_then_http_then_all() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    assert_eq!(auxiliary_proxy_from_env(), None);

    unsafe { std::env::set_var("ALL_PROXY", "http://all:1") };
    assert_eq!(auxiliary_proxy_from_env(), Some("http://all:1".into()));

    unsafe { std::env::set_var("HTTP_PROXY", "http://http:2") };
    assert_eq!(auxiliary_proxy_from_env(), Some("http://http:2".into()));

    unsafe { std::env::set_var("HTTPS_PROXY", "http://https:3") };
    assert_eq!(auxiliary_proxy_from_env(), Some("http://https:3".into()));
}

// Tier: unit — mirrors tests/run_agent/test_create_openai_client_proxy_env.py.
#[test]
fn auxiliary_proxy_from_env_normalizes_socks_alias() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    unsafe { std::env::set_var("ALL_PROXY", "socks://127.0.0.1:1080/") };
    assert_eq!(
        auxiliary_proxy_from_env(),
        Some("socks5://127.0.0.1:1080/".into())
    );
}

// Tier: unit — mirrors tests/agent/test_auxiliary_client_proxy_env.py.
#[test]
fn auxiliary_proxy_for_base_url_respects_no_proxy() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    unsafe {
        std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:7897");
        std::env::set_var("NO_PROXY", "internal.example.com");
    }
    assert_eq!(
        auxiliary_proxy_for_base_url(Some("https://litellm.internal.example.com/v1")),
        None
    );
    assert_eq!(
        auxiliary_proxy_for_base_url(Some("https://api.openai.com/v1")),
        Some("http://127.0.0.1:7897".into())
    );
}

// Tier: unit — mirrors tests/agent/test_ssl_verify.py.
#[test]
fn auxiliary_tls_verify_defaults_to_httpx_certificates() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    assert_eq!(
        resolve_auxiliary_tls_verify(None, None, None),
        AuxiliaryTlsVerify::Default
    );
}

// Tier: unit — mirrors tests/agent/test_ssl_verify.py and
// tests/run_agent/test_create_openai_client_ssl_verify.py.
#[test]
fn auxiliary_tls_verify_uses_existing_ca_bundle_and_explicit_precedence() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();
    let ca_bundle = std::env::current_exe().unwrap();
    let ca_bundle = ca_bundle.to_string_lossy().into_owned();

    unsafe { std::env::set_var("HERMES_CA_BUNDLE", "missing-ca-bundle.pem") };
    assert_eq!(
        resolve_auxiliary_tls_verify(Some(&ca_bundle), None, None),
        AuxiliaryTlsVerify::CaBundle(ca_bundle.clone())
    );

    unsafe { std::env::remove_var("HERMES_CA_BUNDLE") };
    unsafe { std::env::set_var("HERMES_CA_BUNDLE", &ca_bundle) };
    assert_eq!(
        resolve_auxiliary_tls_verify(None, None, None),
        AuxiliaryTlsVerify::CaBundle(ca_bundle)
    );
}

// Tier: unit — mirrors tests/agent/test_auxiliary_client_ssl_verify.py.
#[test]
fn auxiliary_tls_verify_accepts_false_settings_and_fails_open_for_missing_ca() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    assert_eq!(
        resolve_auxiliary_tls_verify(None, Some(&AuxiliarySslVerifySetting::Boolean(false)), None),
        AuxiliaryTlsVerify::Disabled
    );
    assert_eq!(
        resolve_auxiliary_tls_verify(
            None,
            Some(&AuxiliarySslVerifySetting::Text("off".into())),
            None
        ),
        AuxiliaryTlsVerify::Disabled
    );
    assert_eq!(
        resolve_auxiliary_tls_verify(Some("missing-ca-bundle.pem"), None, None),
        AuxiliaryTlsVerify::Default
    );
}

// Tier: unit — mirrors tests/agent/test_codex_cloudflare_headers.py.
#[test]
fn codex_cloudflare_headers_extract_account_id_with_canonical_casing() {
    let token = concat!(
        "e30.",
        "eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjdC10ZXN0LTEyMyJ9fQ",
        ".sig"
    );
    let headers = codex_cloudflare_headers(token);

    assert_eq!(headers.get("originator"), Some(&"codex_cli_rs".into()));
    assert_eq!(
        headers.get("User-Agent"),
        Some(&"codex_cli_rs/0.0.0 (Hermes Agent)".into())
    );
    assert_eq!(
        headers.get("ChatGPT-Account-ID"),
        Some(&"acct-test-123".into())
    );
    assert!(!headers.contains_key("chatgpt-account-id"));
    assert!(!headers.contains_key("ChatGPT-Account-Id"));
}

// Tier: unit — mirrors tests/agent/test_codex_cloudflare_headers.py.
#[test]
fn codex_cloudflare_headers_keep_base_headers_without_account_claim() {
    let token = concat!(
        "e30.",
        "eyJzdWIiOiJ1c2VyLXh5eiIsImV4cCI6OTk5OTk5OTk5OX0",
        ".sig"
    );
    let headers = codex_cloudflare_headers(token);

    assert_eq!(headers.get("originator"), Some(&"codex_cli_rs".into()));
    assert!(!headers.contains_key("ChatGPT-Account-ID"));
}

// Tier: unit — mirrors the source's broad malformed-token fail-open path.
#[test]
fn codex_cloudflare_headers_fail_open_for_malformed_or_empty_tokens() {
    for token in ["", "not-a-jwt", "a.%%%%.c"] {
        let headers = codex_cloudflare_headers(token);
        assert_eq!(headers.get("originator"), Some(&"codex_cli_rs".into()));
        assert!(!headers.contains_key("ChatGPT-Account-ID"));
    }
}

// Tier: unit — mirrors TestReadCodexAccessToken in
// tests/agent/test_auxiliary_client.py.
#[test]
fn codex_access_token_prefers_pool_runtime_key() {
    let entry = AuxiliaryPoolEntry {
        runtime_api_key: Some("  pool-token  ".into()),
        ..AuxiliaryPoolEntry::default()
    };
    let auth = json!({
        "tokens": {"access_token": "auth-token", "refresh_token": "refresh"}
    });

    assert_eq!(
        read_codex_access_token(true, Some(&entry), Some(&auth), 1_700_000_000),
        Some("pool-token".into())
    );
}

// Tier: unit — mirrors TestReadCodexAccessToken in
// tests/agent/test_auxiliary_client.py.
#[test]
fn codex_access_token_reads_and_trims_auth_store_token() {
    let auth = json!({
        "tokens": {"access_token": "  tok-123  ", "refresh_token": "refresh"}
    });

    assert_eq!(
        read_codex_access_token(false, None, Some(&auth), 1_700_000_000),
        Some("tok-123".into())
    );
}

// Tier: unit — mirrors expired_jwt_returns_none and valid_jwt_returns_token.
#[test]
fn codex_access_token_filters_expired_jwt_but_keeps_valid_jwt() {
    let expired = json!({
        "tokens": {"access_token": "h.eyJleHAiOjE3MDAwMDAwMDB9.s"}
    });
    assert_eq!(
        read_codex_access_token(false, None, Some(&expired), 1_700_000_001),
        None
    );

    let valid = json!({
        "tokens": {"access_token": "h.eyJleHAiOjE3MDAwMDAwMDB9.s"}
    });
    assert_eq!(
        read_codex_access_token(false, None, Some(&valid), 1_700_000_000),
        Some("h.eyJleHAiOjE3MDAwMDAwMDB9.s".into())
    );
}

// Tier: unit — mirrors the source's non-JWT decode-error fail-open path.
#[test]
fn codex_access_token_keeps_non_jwt_tokens_and_fails_open_on_missing_shape() {
    let plain = json!({"tokens": {"access_token": "plain-token"}});
    assert_eq!(
        read_codex_access_token(false, None, Some(&plain), 1_700_000_000),
        Some("plain-token".into())
    );
    assert_eq!(
        read_codex_access_token(false, None, Some(&json!({})), 1_700_000_000),
        None
    );
}
// ── Client-level headers and Portal extra_body ──────────────────────────────
// Tier: unit — mirrors tests/agent/test_openrouter_response_cache.py,
// tests/agent/test_user_default_headers.py, and the host-gated header chains
// in agent/auxiliary_client.py. Every test here reads or writes the override
// environment, so all of them take the same mutex.

fn clear_openrouter_env() {
    unsafe {
        std::env::remove_var("HERMES_OPENROUTER_CACHE");
        std::env::remove_var("HERMES_OPENROUTER_CACHE_TTL");
    }
}

fn or_section(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    value.as_object().expect("openrouter section").clone()
}

#[test]
fn openrouter_headers_default_to_enabled_ttl_300() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    clear_openrouter_env();
    let headers = build_or_headers(None);
    assert_eq!(
        headers.get("X-OpenRouter-Cache").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        headers.get("X-OpenRouter-Cache-TTL").map(String::as_str),
        Some("300")
    );
    assert_eq!(
        headers.get("HTTP-Referer").map(String::as_str),
        Some("https://hermes-agent.nousresearch.com")
    );
    assert_eq!(
        headers.get("X-Title").map(String::as_str),
        Some("Hermes Agent")
    );
    assert_eq!(
        headers.get("X-OpenRouter-Categories").map(String::as_str),
        Some("productivity,cli-agent")
    );
    // `TestDefaultConfig::test_openrouter_section_exists`: the defaults that
    // back this fallback carry the openrouter section.
    let defaults = hermes_agent::config::openrouter_defaults();
    assert_eq!(defaults["openrouter"]["response_cache"], json!(true));
    assert_eq!(defaults["openrouter"]["response_cache_ttl"], json!(300));
    // `test_returns_fresh_dict`: each call returns an owned map, so mutating
    // one result cannot leak into the next.
    let mut first = build_or_headers(None);
    first.insert("X-OpenRouter-Cache".into(), "mutated".into());
    assert_eq!(
        build_or_headers(None)
            .get("X-OpenRouter-Cache")
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn openrouter_headers_ttl_default_and_bounds() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    clear_openrouter_env();
    // test_ttl_default
    let headers = build_or_headers(Some(&or_section(json!(
        {"response_cache": true, "response_cache_ttl": 300}
    ))));
    assert_eq!(
        headers.get("X-OpenRouter-Cache-TTL").map(String::as_str),
        Some("300")
    );
    // test_ttl_negative: the cache header survives, the TTL header is dropped.
    let headers = build_or_headers(Some(&or_section(json!(
        {"response_cache": true, "response_cache_ttl": -5}
    ))));
    assert_eq!(
        headers.get("X-OpenRouter-Cache").map(String::as_str),
        Some("true")
    );
    assert!(!headers.contains_key("X-OpenRouter-Cache-TTL"));
    // Out-of-range and non-numeric TTLs are dropped the same way.
    let headers = build_or_headers(Some(&or_section(json!(
        {"response_cache": true, "response_cache_ttl": 86401}
    ))));
    assert!(!headers.contains_key("X-OpenRouter-Cache-TTL"));
    let headers = build_or_headers(Some(&or_section(json!(
        {"response_cache": true, "response_cache_ttl": "300"}
    ))));
    assert!(!headers.contains_key("X-OpenRouter-Cache-TTL"));
    // Python truthiness: an empty string disables the cache header entirely.
    let headers = build_or_headers(Some(&or_section(json!({"response_cache": ""}))));
    assert!(!headers.contains_key("X-OpenRouter-Cache"));
    assert_eq!(headers.len(), 3);
}

#[test]
fn openrouter_headers_empty_section_disables_cache() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    clear_openrouter_env();
    // `or_config={}`: no `response_cache` key means the default is False.
    let headers = build_or_headers(Some(&serde_json::Map::new()));
    assert!(!headers.contains_key("X-OpenRouter-Cache"));
    assert_eq!(headers.len(), 3);
}

#[test]
fn openrouter_headers_invalid_env_ttl_dropped() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    for ttl in ["0", "86401", "abc", "-1", "12.5"] {
        unsafe {
            std::env::set_var("HERMES_OPENROUTER_CACHE", "1");
            std::env::set_var("HERMES_OPENROUTER_CACHE_TTL", ttl);
        }
        let headers = build_or_headers(Some(&serde_json::Map::new()));
        assert_eq!(
            headers.get("X-OpenRouter-Cache").map(String::as_str),
            Some("true")
        );
        assert!(
            !headers.contains_key("X-OpenRouter-Cache-TTL"),
            "ttl {ttl} must be dropped"
        );
    }
    clear_openrouter_env();
}

#[test]
fn openrouter_headers_valid_env_ttl_boundaries() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    for ttl in ["1", "300", "86400"] {
        unsafe {
            std::env::set_var("HERMES_OPENROUTER_CACHE", "yes");
            std::env::set_var("HERMES_OPENROUTER_CACHE_TTL", ttl);
        }
        let headers = build_or_headers(Some(&serde_json::Map::new()));
        assert_eq!(
            headers.get("X-OpenRouter-Cache-TTL").map(String::as_str),
            Some(ttl)
        );
    }
    clear_openrouter_env();
}

#[test]
fn openrouter_headers_falsy_env_beats_enabled_config() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    unsafe { std::env::set_var("HERMES_OPENROUTER_CACHE", "0") };
    let headers = build_or_headers(Some(&or_section(json!({"response_cache": true}))));
    clear_openrouter_env();
    assert!(!headers.contains_key("X-OpenRouter-Cache"));
}

#[test]
fn openrouter_route_gate_covers_both_upstream_sites() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    clear_openrouter_env();
    // A non-openrouter route with a non-openrouter host carries nothing.
    assert!(
        openrouter_cache_headers(Some("other"), Some("https://example.com/v1"), None).is_empty()
    );
    assert!(openrouter_cache_headers(None, None, None).is_empty());
    // `_try_openrouter`: the openrouter route attaches the headers whatever the
    // resolved base URL is (a pool proxy included).
    let headers =
        openrouter_cache_headers(Some(" OpenRouter "), Some("https://proxy.example/v1"), None);
    assert_eq!(
        headers.get("X-OpenRouter-Cache").map(String::as_str),
        Some("true")
    );
    // `_to_async_client`: any client whose host is openrouter.ai gets them too.
    let headers =
        openrouter_cache_headers(Some("custom"), Some("https://openrouter.ai/api/v1"), None);
    assert_eq!(
        headers.get("X-OpenRouter-Cache").map(String::as_str),
        Some("true")
    );
    // A supplied `openrouter` section is used as-is; an empty one disables.
    let headers = openrouter_cache_headers(
        Some("openrouter"),
        None,
        Some(&or_section(
            json!({"response_cache": true, "response_cache_ttl": 60}),
        )),
    );
    assert!(
        !openrouter_cache_headers(Some("openrouter"), None, Some(&serde_json::Map::new()))
            .contains_key("X-OpenRouter-Cache")
    );
    assert_eq!(
        headers.get("X-OpenRouter-Cache-TTL").map(String::as_str),
        Some("60")
    );
}

#[test]
fn provider_default_headers_chain_kimi_copilot_nvidia_and_profile() {
    let headers =
        resolve_provider_default_headers(None, Some("https://api.kimi.com/coding/v1"), false);
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("claude-code/0.1.0")
    );

    let headers = resolve_provider_default_headers(None, Some("https://githubcopilot.com"), true);
    assert_eq!(
        headers.get("Editor-Version").map(String::as_str),
        Some("vscode/1.104.1")
    );
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("HermesAgent/1.0")
    );
    assert_eq!(
        headers.get("Copilot-Integration-Id").map(String::as_str),
        Some("vscode-chat")
    );
    assert_eq!(
        headers.get("Openai-Intent").map(String::as_str),
        Some("conversation-edits")
    );
    assert_eq!(
        headers.get("x-initiator").map(String::as_str),
        Some("agent")
    );
    assert_eq!(
        headers.get("Copilot-Vision-Request").map(String::as_str),
        Some("true")
    );

    let headers =
        resolve_provider_default_headers(None, Some("https://integrate.api.nvidia.com/v1"), false);
    assert_eq!(
        headers.get("X-BILLING-INVOKE-ORIGIN").map(String::as_str),
        Some("HermesAgent")
    );

    // Profile default headers apply for non-special hosts; an empty base
    // carries none (the `if custom_base and custom_key` guard).
    let headers = resolve_provider_default_headers(
        Some("ai-gateway"),
        Some("https://ai-gateway.vercel.sh/v1"),
        false,
    );
    assert_eq!(
        headers.get("X-Title").map(String::as_str),
        Some("Hermes Agent")
    );
    assert_eq!(
        headers.get("HTTP-Referer").map(String::as_str),
        Some("https://hermes-agent.nousresearch.com")
    );
    assert!(resolve_provider_default_headers(Some("ai-gateway"), None, false).is_empty());
    assert!(resolve_provider_default_headers(
        Some("no-such-provider"),
        Some("https://example.com"),
        false
    )
    .is_empty());
}

#[test]
fn copilot_headers_defaults_and_vision_toggle() {
    let headers = copilot_request_headers(true, false);
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("HermesAgent/1.0")
    );
    assert_eq!(
        headers.get("x-initiator").map(String::as_str),
        Some("agent")
    );
    assert!(!headers.contains_key("Copilot-Vision-Request"));
    assert_eq!(
        copilot_request_headers(false, false)
            .get("x-initiator")
            .map(String::as_str),
        Some("user")
    );
    assert_eq!(copilot_request_headers(false, false).len(), 5);
}

#[test]
fn nvidia_nim_headers_are_host_gated() {
    assert_eq!(
        build_nvidia_nim_headers(Some("https://integrate.api.nvidia.com/v1")).len(),
        1
    );
    assert!(build_nvidia_nim_headers(Some("https://build.nvidia.com")).is_empty());
    assert!(build_nvidia_nim_headers(Some("https://example.com")).is_empty());
    assert!(build_nvidia_nim_headers(None).is_empty());
}

#[test]
fn user_default_headers_win_over_provider_defaults() {
    let config = or_section(json!({
        "model": {"default_headers": {
            "X-Custom": "1",
            "X-Bool": true,
            "X-Null": null,
            "X-Number": 2.5,
            "  ": "blank key survives",
            "User-Agent": "curl/8.7.1",
        }}
    }));
    let mut headers = BTreeMap::from([
        ("User-Agent".to_string(), "OpenAI/Python 1.0".to_string()),
        ("X-Keep".to_string(), "provider".to_string()),
    ]);
    apply_user_default_headers(&mut headers, Some(&config));

    // User values win; untouched provider values survive.
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("curl/8.7.1")
    );
    assert_eq!(headers.get("X-Keep").map(String::as_str), Some("provider"));
    assert_eq!(headers.get("X-Custom").map(String::as_str), Some("1"));
    // Non-string scalars stringify with Python casing.
    assert_eq!(headers.get("X-Bool").map(String::as_str), Some("True"));
    assert_eq!(headers.get("X-Number").map(String::as_str), Some("2.5"));
    // Only an explicit null is skipped, and keys are never trimmed.
    assert!(!headers.contains_key("X-Null"));
    assert_eq!(
        headers.get("  ").map(String::as_str),
        Some("blank key survives")
    );
}

#[test]
fn user_extra_headers_alias_wins_over_default_headers() {
    let config = or_section(json!({
        "model": {
            "default_headers": {"X-A": "from-default", "X-B": "only-default"},
            "extra_headers": {"X-A": "from-alias"},
        }
    }));
    let mut headers = BTreeMap::new();
    apply_user_default_headers(&mut headers, Some(&config));

    assert_eq!(headers.get("X-A").map(String::as_str), Some("from-alias"));
    assert_eq!(headers.get("X-B").map(String::as_str), Some("only-default"));
}

#[test]
fn user_headers_absent_or_non_mapping_is_a_noop() {
    let mut headers = BTreeMap::from([("X-Keep".to_string(), "provider".to_string())]);
    apply_user_default_headers(
        &mut headers,
        Some(&or_section(json!({"model": "oops_a_string"}))),
    );
    apply_user_default_headers(
        &mut headers,
        Some(&or_section(json!({"model": {"default_headers": []}}))),
    );
    apply_user_default_headers(&mut headers, Some(&serde_json::Map::new()));
    assert_eq!(
        headers,
        BTreeMap::from([("X-Keep".to_string(), "provider".to_string())])
    );
}

#[test]
fn auxiliary_default_headers_compose_in_source_order() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    clear_openrouter_env();
    let config = or_section(json!({
        "openrouter": {"response_cache": true, "response_cache_ttl": 45},
        "model": {"default_headers": {"X-Title": "user wins"}},
    }));
    let headers = auxiliary_default_headers(
        Some("openrouter"),
        Some("https://openrouter.ai/api/v1"),
        false,
        Some(&config),
    );
    // Provider/base attribution, then the OpenRouter cache headers, then the
    // user overlay on top.
    assert_eq!(
        headers.get("X-OpenRouter-Cache-TTL").map(String::as_str),
        Some("45")
    );
    assert_eq!(
        headers.get("X-OpenRouter-Categories").map(String::as_str),
        Some("productivity,cli-agent")
    );
    assert_eq!(
        headers.get("X-Title").map(String::as_str),
        Some("user wins")
    );
}

#[test]
fn auxiliary_extra_body_and_portal_fallback_track_the_ambient_context() {
    use hermes_agent::portal_tags::{conversation_tag, set_conversation_context};

    let token = set_conversation_context(Some("conv-9"));
    assert!(get_auxiliary_extra_body(false).is_empty());
    let extra = get_auxiliary_extra_body(true);
    let tags = extra["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[2], json!("conversation=conv-9"));
    // `nous_extra_body` is the same document the Portal fallback reuses.
    assert_eq!(nous_extra_body(), extra);

    // Fallback entries only for nous spellings and only for absent keys.
    let fallback = nous_portal_fallback_extra(Some(" Nous "), false, false);
    assert_eq!(fallback["tags"].as_array().expect("tags").len(), 3);
    assert_eq!(fallback["session_id"], json!("conv-9"));
    assert!(nous_portal_fallback_extra(Some("nousresearch"), true, true).is_empty());
    assert!(nous_portal_fallback_extra(Some("openrouter"), false, false).is_empty());
    let only_tags = nous_portal_fallback_extra(Some("nous-portal"), true, false);
    assert!(!only_tags.contains_key("tags"));
    assert_eq!(only_tags["session_id"], json!("conv-9"));

    set_conversation_context(None);
    let fallback = nous_portal_fallback_extra(Some("nous"), false, true);
    assert!(fallback.contains_key("tags"));
    assert!(!fallback.contains_key("session_id"));
    assert!(fallback["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .all(|tag| tag
            .as_str()
            .is_some_and(|tag| !tag.starts_with("conversation="))));
    // The base tags are freshly computed, not a leaked snapshot.
    let tags = nous_portal_fallback_extra(Some("nous"), false, false)["tags"].clone();
    assert_eq!(tags.as_array().expect("tags").len(), 2);
    set_conversation_context(token.as_deref());
    let _ = conversation_tag("unused");
}

// Tier: unit — mirrors the two `logger.warning` calls the source emits inside
// `resolve_httpx_verify` (agent/ssl_verify.py lines 40-44 and 58-61).
#[test]
fn auxiliary_tls_verify_warnings_match_the_source_text() {
    let _lock = AUXILIARY_ENV_MUTEX.lock();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    let (verify, warning) = auxiliary_tls_verify_resolution(
        None,
        Some(&AuxiliarySslVerifySetting::Boolean(false)),
        Some("http://127.0.0.1:11434/v1"),
    );
    assert_eq!(verify, AuxiliaryTlsVerify::Disabled);
    assert_eq!(
        warning.expect("insecure warning").text(),
        "TLS certificate verification DISABLED (ssl_verify: false) for \
         http://127.0.0.1:11434/v1 — this is intended for local development only \
         and is unsafe on any network you do not fully control."
    );

    // No endpoint still yields the source's placeholder.
    let (_, warning) = auxiliary_tls_verify_resolution(
        None,
        Some(&AuxiliarySslVerifySetting::Text("  OFF ".into())),
        None,
    );
    assert!(warning
        .expect("insecure warning")
        .text()
        .contains("for a custom provider endpoint —"));

    let (verify, warning) =
        auxiliary_tls_verify_resolution(Some("missing-ca-bundle.pem"), None, None);
    assert_eq!(verify, AuxiliaryTlsVerify::Default);
    assert_eq!(
        warning.expect("missing bundle warning").text(),
        "CA bundle path does not exist: missing-ca-bundle.pem — falling back to \
         default certificates"
    );

    // A resolved bundle logs nothing.
    unsafe { std::env::remove_var("HERMES_CA_BUNDLE") };
    assert!(auxiliary_tls_verify_resolution(None, None, None)
        .1
        .is_none());
}
