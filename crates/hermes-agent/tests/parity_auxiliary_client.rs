use hermes_agent::auxiliary_client::{
    auxiliary_http_client_config, auxiliary_max_tokens_param, auxiliary_proxy_for_base_url,
    auxiliary_proxy_from_env, build_auxiliary_http_client, codex_cloudflare_headers,
    is_anthropic_compatible_host, is_model_incompatible_error, is_model_not_found_error,
    is_payment_error, is_rate_limit_error, normalize_aux_provider, openai_client_config,
    openai_client_config_with_transport, pool_runtime_api_key, pool_runtime_base_url,
    read_codex_access_token, resolve_aux_task_provider_model, resolve_auxiliary_tls_verify,
    resolve_pool_first_runtime_credentials, select_auxiliary_pool_entry, to_openai_base_url,
    AuxiliaryError, AuxiliaryHttpClient, AuxiliaryHttpClientConfig, AuxiliaryPoolEntry,
    AuxiliaryRuntimeCredentials, AuxiliarySslVerifySetting, AuxiliaryTaskConfig,
    AuxiliaryTlsVerify,
};
use serde_json::json;
use std::ffi::OsString;
use std::sync::Mutex;

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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
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
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    assert_eq!(
        resolve_auxiliary_tls_verify(None, None),
        AuxiliaryTlsVerify::Default
    );
}

// Tier: unit — mirrors tests/agent/test_ssl_verify.py and
// tests/run_agent/test_create_openai_client_ssl_verify.py.
#[test]
fn auxiliary_tls_verify_uses_existing_ca_bundle_and_explicit_precedence() {
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();
    let ca_bundle = std::env::current_exe().unwrap();
    let ca_bundle = ca_bundle.to_string_lossy().into_owned();

    unsafe { std::env::set_var("HERMES_CA_BUNDLE", "missing-ca-bundle.pem") };
    assert_eq!(
        resolve_auxiliary_tls_verify(Some(&ca_bundle), None),
        AuxiliaryTlsVerify::CaBundle(ca_bundle.clone())
    );

    unsafe { std::env::remove_var("HERMES_CA_BUNDLE") };
    unsafe { std::env::set_var("HERMES_CA_BUNDLE", &ca_bundle) };
    assert_eq!(
        resolve_auxiliary_tls_verify(None, None),
        AuxiliaryTlsVerify::CaBundle(ca_bundle)
    );
}

// Tier: unit — mirrors tests/agent/test_auxiliary_client_ssl_verify.py.
#[test]
fn auxiliary_tls_verify_accepts_false_settings_and_fails_open_for_missing_ca() {
    let _lock = AUXILIARY_ENV_MUTEX.lock().unwrap();
    let _environment = EnvironmentSnapshot::new();
    clear_auxiliary_env();

    assert_eq!(
        resolve_auxiliary_tls_verify(None, Some(&AuxiliarySslVerifySetting::Boolean(false)),),
        AuxiliaryTlsVerify::Disabled
    );
    assert_eq!(
        resolve_auxiliary_tls_verify(None, Some(&AuxiliarySslVerifySetting::Text("off".into())),),
        AuxiliaryTlsVerify::Disabled
    );
    assert_eq!(
        resolve_auxiliary_tls_verify(Some("missing-ca-bundle.pem"), None),
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
