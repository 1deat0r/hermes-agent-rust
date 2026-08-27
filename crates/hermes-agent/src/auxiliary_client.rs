//! Dependency-safe parity helpers from `agent/auxiliary_client.py`.
//!
//! This module starts with pure routing predicates and wire parameter helpers.
//! Transport-neutral SDK/httpx options and pool-first credential projection are
//! now included; concrete SDK/network clients, pool lifecycle/rotation,
//! cancellation, and provider fallback chains remain higher-layer sections of
//! the 10,044-line upstream module.

use crate::config::{cfg_get, json_truthy, load_merged_config_snapshot, openrouter_defaults};
use crate::portal_tags::get_conversation_context;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use hermes_providers::registry::get_provider_profile;
use hermes_utils::{
    base_url_host_matches, base_url_hostname, model_forces_max_completion_tokens,
    normalize_proxy_url,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

/// Minimal exception-shaped adapter used by the source error classifiers.
///
/// Python receives arbitrary SDK exceptions and reads status_code, the runtime
/// class name, and str(exc). Rust callers pass those three pieces explicitly
/// until the transport crate supplies its concrete error types.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct AuxiliaryError {
    pub status_code: Option<u16>,
    pub type_name: String,
    pub message: String,
}

impl AuxiliaryError {
    pub fn new(status_code: Option<u16>, type_name: &str, message: &str) -> Self {
        Self {
            status_code,
            type_name: type_name.into(),
            message: message.into(),
        }
    }
}

const PROVIDER_ALIASES: &[(&str, &str)] = &[
    ("google", "gemini"),
    ("google-gemini", "gemini"),
    ("google-ai-studio", "gemini"),
    ("x-ai", "xai"),
    ("x.ai", "xai"),
    ("grok", "xai"),
    ("glm", "zai"),
    ("z-ai", "zai"),
    ("z.ai", "zai"),
    ("zhipu", "zai"),
    ("kimi", "kimi-coding"),
    ("moonshot", "kimi-coding"),
    ("kimi-cn", "kimi-coding-cn"),
    ("moonshot-cn", "kimi-coding-cn"),
    ("gmi-cloud", "gmi"),
    ("gmicloud", "gmi"),
    ("actual-computer", "actual"),
    ("actualcomputer", "actual"),
    ("aci", "actual"),
    ("minimax-china", "minimax-cn"),
    ("minimax_cn", "minimax-cn"),
    ("claude", "anthropic"),
    ("claude-code", "anthropic"),
    ("github", "copilot"),
    ("github-copilot", "copilot"),
    ("github-model", "copilot"),
    ("github-models", "copilot"),
    ("github-copilot-acp", "copilot-acp"),
    ("copilot-acp-agent", "copilot-acp"),
    ("tencent", "tencent-tokenhub"),
    ("tokenhub", "tencent-tokenhub"),
    ("tencent-cloud", "tencent-tokenhub"),
    ("tencentmaas", "tencent-tokenhub"),
];

/// The config fields consumed by the dependency-safe task-provider resolver.
///
/// The Python source obtains this map from `load_config_readonly()` and also
/// resolves `key_env` through the secret scope. Rust callers supply the
/// already-resolved values here; credential-pool and environment resolution
/// remain higher-layer seams.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuxiliaryTaskConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_mode: Option<String>,
}

/// Result of resolving one auxiliary task's provider/model/endpoint inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuxiliaryTaskProviderResolution {
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_mode: Option<String>,
}

fn trimmed_option(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalized_model(value: Option<&str>) -> Option<String> {
    let value = trimmed_option(value)?;
    if value.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(value)
    }
}

/// Temperature behavior required by an auxiliary model contract.
///
/// `Default` preserves the caller's temperature behavior, `Omit` removes the
/// parameter so the provider selects it, and `Fixed` replaces it with a
/// specific value.
#[derive(Debug, Clone, PartialEq)]
pub enum AuxiliaryTemperaturePolicy {
    Default,
    Omit,
    Fixed(f64),
}

fn auxiliary_model_suffix(model: Option<&str>) -> String {
    model
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_owned()
}

/// Return whether a model belongs to the Kimi family.
///
/// PARITY: agent/auxiliary_client.py lines 560-563.
pub fn is_kimi_model(model: Option<&str>) -> bool {
    let bare = auxiliary_model_suffix(model);
    bare == "kimi" || bare.starts_with("kimi-")
}

/// Return whether a model is exactly Arcee Trinity Large Thinking.
///
/// PARITY: agent/auxiliary_client.py lines 566-570.
pub fn is_arcee_trinity_thinking(model: Option<&str>) -> bool {
    auxiliary_model_suffix(model) == "trinity-large-thinking"
}

/// Return whether a model is a gpt-5.4/5.5/5.6 Codex OAuth variant.
///
/// The historical source name is retained even though gpt-5.6 is included.
///
/// PARITY: agent/auxiliary_client.py lines 596-622.
pub fn is_codex_gpt54_or_gpt55(model: Option<&str>, provider: Option<&str>) -> bool {
    let provider = provider.unwrap_or("").trim().to_ascii_lowercase();
    if provider != "openai-codex" {
        return false;
    }
    let bare = auxiliary_model_suffix(model);
    ["gpt-5.4", "gpt-5.5", "gpt-5.6"].iter().any(|family| {
        bare == *family
            || bare
                .strip_prefix(family)
                .is_some_and(|suffix| suffix.starts_with('-') || suffix.starts_with('.'))
    })
}

/// Return whether a model is the exact Codex OAuth Spark model.
///
/// PARITY: agent/auxiliary_client.py lines 625-637.
pub fn is_codex_spark(model: Option<&str>, provider: Option<&str>) -> bool {
    let provider = provider.unwrap_or("").trim().to_ascii_lowercase();
    provider == "openai-codex" && auxiliary_model_suffix(model) == "gpt-5.3-codex-spark"
}

/// Resolve the source's model-specific temperature directive.
///
/// The base URL is accepted to preserve the source helper's call shape; these
/// policies depend only on the normalized model.
///
/// PARITY: agent/auxiliary_client.py lines 640-659.
pub fn fixed_temperature_for_model(
    model: Option<&str>,
    _base_url: Option<&str>,
) -> AuxiliaryTemperaturePolicy {
    if is_kimi_model(model) {
        AuxiliaryTemperaturePolicy::Omit
    } else if is_arcee_trinity_thinking(model) {
        AuxiliaryTemperaturePolicy::Fixed(0.5)
    } else {
        AuxiliaryTemperaturePolicy::Default
    }
}

/// Return a model/route-specific context-compression threshold override.
///
/// `None` leaves the caller's configured threshold unchanged.
///
/// PARITY: agent/auxiliary_client.py lines 662-697.
pub fn compression_threshold_for_model(
    model: Option<&str>,
    provider: Option<&str>,
    allow_codex_gpt55_autoraise: bool,
) -> Option<f64> {
    if is_arcee_trinity_thinking(model) {
        Some(0.75)
    } else if allow_codex_gpt55_autoraise && is_codex_gpt54_or_gpt55(model, provider) {
        Some(0.85)
    } else if is_codex_spark(model, provider) {
        Some(0.70)
    } else {
        None
    }
}

fn is_known_aux_provider(provider: &str, known_provider_ids: &[&str]) -> bool {
    known_provider_ids
        .iter()
        .any(|known| known.eq_ignore_ascii_case(provider))
}

/// Resolve explicit and per-task provider/model/endpoint inputs.
///
/// `task_config` is the source's `auxiliary.<task>` map, `moa_aggregator` is
/// the already-resolved `(provider, model)` for a MoA preset, and
/// `known_provider_ids` is the provider registry view used to preserve
/// first-class provider identity when an explicit base URL is present. These
/// are explicit Rust adapters for Python config/registry lookups.
///
/// PARITY: agent/auxiliary_client.py lines 7369-7558.
pub fn resolve_aux_task_provider_model(
    task_config: Option<&AuxiliaryTaskConfig>,
    task: Option<&str>,
    explicit_provider: Option<&str>,
    explicit_model: Option<&str>,
    explicit_base_url: Option<&str>,
    explicit_api_key: Option<&str>,
    moa_aggregator: Option<(&str, &str)>,
    known_provider_ids: &[&str],
) -> AuxiliaryTaskProviderResolution {
    let mut cfg_provider =
        trimmed_option(task_config.and_then(|config| config.provider.as_deref()));
    let cfg_model = normalized_model(task_config.and_then(|config| config.model.as_deref()));
    let mut cfg_base_url =
        trimmed_option(task_config.and_then(|config| config.base_url.as_deref()));
    let mut cfg_api_key = trimmed_option(task_config.and_then(|config| config.api_key.as_deref()));
    let cfg_api_mode = trimmed_option(task_config.and_then(|config| config.api_mode.as_deref()));

    let mut provider = trimmed_option(explicit_provider);
    let mut model = normalized_model(explicit_model).or(cfg_model.clone());
    let mut base_url = trimmed_option(explicit_base_url);
    let mut api_key = trimmed_option(explicit_api_key);
    let task_present = task.is_some_and(|name| !name.trim().is_empty());

    let unwrap_moa = |candidate: &str, candidate_model: Option<String>| {
        if !candidate.eq_ignore_ascii_case("moa") {
            return (candidate.to_owned(), candidate_model, false);
        }
        match moa_aggregator {
            Some((aggregator_provider, aggregator_model))
                if !aggregator_provider.trim().is_empty()
                    && !aggregator_model.trim().is_empty()
                    && !aggregator_provider.eq_ignore_ascii_case("moa") =>
            {
                (
                    aggregator_provider.trim().to_owned(),
                    Some(aggregator_model.trim().to_owned()),
                    true,
                )
            }
            _ => (candidate.to_owned(), candidate_model, false),
        }
    };

    // An explicit provider takes the same MoA chokepoint as the source. The
    // virtual endpoint/key belong to the facade and must not reach the real
    // aggregator client after a successful unwrap.
    if let Some(candidate) = provider.take() {
        let (resolved_provider, resolved_model, unwrapped) = unwrap_moa(&candidate, model);
        provider = Some(resolved_provider);
        model = resolved_model;
        if unwrapped {
            base_url = None;
            api_key = None;
        }
    } else if let Some(candidate) = cfg_provider.clone() {
        // The config path passes the already-selected model to the shared
        // MoA resolver, then clears config endpoint credentials on success.
        let (resolved_provider, resolved_model, unwrapped) = unwrap_moa(&candidate, model.clone());
        if unwrapped {
            model = resolved_model;
            cfg_base_url = None;
            cfg_api_key = None;
        }
        cfg_provider = Some(resolved_provider);
    }

    // Direct API-key aliases are not registry providers. Preserve a caller's
    // endpoint when present; otherwise use the source's OpenAI default.
    let expand_direct_alias =
        |candidate: Option<String>, existing_base: Option<String>| match candidate {
            Some(value) if value.eq_ignore_ascii_case("openai") => (
                Some("custom".to_owned()),
                existing_base.or_else(|| Some("https://api.openai.com/v1".to_owned())),
            ),
            other => (other, existing_base),
        };
    let (expanded_provider, expanded_base_url) = expand_direct_alias(provider, base_url);
    provider = expanded_provider;
    base_url = expanded_base_url;
    let (expanded_cfg_provider, expanded_cfg_base_url) =
        expand_direct_alias(cfg_provider, cfg_base_url);
    cfg_provider = expanded_cfg_provider;
    cfg_base_url = expanded_cfg_base_url;

    // An explicit provider may use the task's endpoint/key when the task names
    // the same provider (or leaves the provider unspecified).
    if let Some(provider_value) = provider.as_ref() {
        if !provider_value.eq_ignore_ascii_case("auto")
            && base_url.is_none()
            && cfg_base_url.is_some()
            && (cfg_provider.is_none()
                || cfg_provider
                    .as_deref()
                    .is_some_and(|configured| configured.eq_ignore_ascii_case(provider_value)))
        {
            base_url = cfg_base_url.clone();
            if api_key.is_none() {
                api_key = cfg_api_key.clone();
            }
        }
    }

    if let Some(url) = base_url {
        let preserve = provider.as_deref().is_some_and(|value| {
            let lowered = value.to_ascii_lowercase();
            !lowered.is_empty()
                && lowered != "auto"
                && lowered != "custom"
                && !lowered.starts_with("custom:")
                && is_known_aux_provider(&lowered, known_provider_ids)
        });
        return AuxiliaryTaskProviderResolution {
            provider: if preserve {
                provider.unwrap_or_else(|| "custom".into())
            } else {
                "custom".into()
            },
            model,
            base_url: Some(url),
            api_key,
            api_mode: cfg_api_mode,
        };
    }

    if let Some(provider) = provider {
        return AuxiliaryTaskProviderResolution {
            provider,
            model,
            base_url: None,
            api_key,
            api_mode: cfg_api_mode,
        };
    }

    if task_present || task_config.is_some() {
        if cfg_base_url.is_some() && cfg_api_key.is_some() {
            return AuxiliaryTaskProviderResolution {
                provider: "custom".into(),
                model,
                base_url: cfg_base_url,
                api_key: cfg_api_key,
                api_mode: cfg_api_mode,
            };
        }
        if let (Some(url), Some(configured_provider)) = (cfg_base_url.clone(), cfg_provider.clone())
        {
            if !configured_provider.eq_ignore_ascii_case("auto") {
                return AuxiliaryTaskProviderResolution {
                    provider: configured_provider,
                    model,
                    base_url: Some(url),
                    api_key: None,
                    api_mode: cfg_api_mode,
                };
            }
        }
        if let Some(configured_provider) = cfg_provider {
            if !configured_provider.eq_ignore_ascii_case("auto") {
                return AuxiliaryTaskProviderResolution {
                    provider: configured_provider,
                    model,
                    base_url: cfg_base_url,
                    api_key: cfg_api_key,
                    api_mode: cfg_api_mode,
                };
            }
        }
    }

    AuxiliaryTaskProviderResolution {
        provider: "auto".into(),
        model,
        base_url: None,
        api_key: None,
        api_mode: cfg_api_mode,
    }
}

/// The subset of a credential-pool entry consumed by auxiliary client setup.
///
/// `runtime_api_key` and `runtime_base_url` are already projected by the
/// credential pool. For Nous, the pool's JWT validation and
/// `inference_base_url` selection happen in the auth layer; callers pass the
/// resulting values here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuxiliaryPoolEntry {
    pub provider: Option<String>,
    pub runtime_api_key: Option<String>,
    pub access_token: Option<String>,
    pub runtime_base_url: Option<String>,
    pub inference_base_url: Option<String>,
    pub base_url: Option<String>,
}

fn normalize_pool_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_owned()
}

/// Resolve the runtime credential used by auxiliary client construction.
///
/// The source first asks the pool entry for its provider-aware
/// `runtime_api_key`, then falls back to the raw `access_token`, and finally
/// strips whitespace from the selected value.
///
/// PARITY: agent/auxiliary_client.py lines 1079-1086.
pub fn pool_runtime_api_key(entry: Option<&AuxiliaryPoolEntry>) -> String {
    let Some(entry) = entry else {
        return String::new();
    };
    let selected = entry
        .runtime_api_key
        .as_deref()
        .filter(|value| !value.is_empty())
        .or(entry.access_token.as_deref())
        .unwrap_or_default();
    selected.trim().to_owned()
}

/// Resolve and normalize the runtime base URL used by auxiliary clients.
///
/// Nous's explicit inference override is checked before the projected pool
/// URL. All other providers use runtime URL, inference URL, base URL, then
/// caller fallback, in that order.
///
/// `nous_inference_override` is the explicit adapter for the source's
/// `_nous_inference_env_override()` lookup.
///
/// PARITY: agent/auxiliary_client.py lines 1088-1118.
pub fn pool_runtime_base_url(
    entry: Option<&AuxiliaryPoolEntry>,
    fallback: Option<&str>,
    nous_inference_override: Option<&str>,
) -> String {
    let Some(entry) = entry else {
        return normalize_pool_url(fallback.unwrap_or_default());
    };

    if entry.provider.as_deref() == Some("nous") {
        if let Some(override_url) = nous_inference_override.filter(|value| !value.is_empty()) {
            return normalize_pool_url(override_url);
        }
    }

    let selected = entry
        .runtime_base_url
        .as_deref()
        .filter(|value| !value.is_empty())
        .or(entry.inference_base_url.as_deref())
        .filter(|value| !value.is_empty())
        .or(entry.base_url.as_deref())
        .filter(|value| !value.is_empty())
        .or(fallback)
        .unwrap_or_default();
    normalize_pool_url(selected)
}

/// Result of selecting a provider credential from the source pool boundary.
///
/// The Python pool is loaded dynamically and can fail open in two distinct
/// ways: no usable pool exists, or a present pool cannot currently select an
/// entry. Keeping the presence bit separate from the optional entry preserves
/// the distinction used by Codex/Nous fallback logic.
///
/// PARITY: agent/auxiliary_client.py lines 1040-1076.
pub fn select_auxiliary_pool_entry<'a>(
    pool_load_succeeded: bool,
    pool_has_credentials: bool,
    selected_entry: Option<&'a AuxiliaryPoolEntry>,
) -> (bool, Option<&'a AuxiliaryPoolEntry>) {
    if !pool_load_succeeded || !pool_has_credentials {
        return (false, None);
    }
    (true, selected_entry)
}

/// Runtime credential pair projected from a pool or legacy auth resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliaryRuntimeCredentials {
    pub api_key: String,
    pub base_url: String,
}

/// Resolve auxiliary runtime credentials pool-first, then through a legacy
/// auth-store/runtime resolver supplied by the caller.
///
/// A present pool with no selectable/usable entry does not suppress the
/// legacy resolver for runtime calls: the source's Nous and xAI OAuth helpers
/// both fall through to their singleton auth resolver in that case. URL
/// normalization is shared with `pool_runtime_base_url` so provider-specific
/// Nous overrides retain their precedence.
///
/// PARITY: agent/auxiliary_client.py lines 2195-2269.
pub fn resolve_pool_first_runtime_credentials(
    pool_present: bool,
    pool_entry: Option<&AuxiliaryPoolEntry>,
    pool_base_url_fallback: Option<&str>,
    nous_inference_override: Option<&str>,
    legacy: Option<(&str, &str)>,
) -> Option<AuxiliaryRuntimeCredentials> {
    if pool_present {
        let api_key = pool_runtime_api_key(pool_entry);
        let base_url =
            pool_runtime_base_url(pool_entry, pool_base_url_fallback, nous_inference_override);
        if !api_key.is_empty() && !base_url.is_empty() {
            return Some(AuxiliaryRuntimeCredentials { api_key, base_url });
        }
    }

    let (api_key, base_url) = legacy?;
    let api_key = api_key.trim();
    let base_url = normalize_pool_url(base_url);
    if api_key.is_empty() || base_url.is_empty() {
        return None;
    }
    Some(AuxiliaryRuntimeCredentials {
        api_key: api_key.to_owned(),
        base_url,
    })
}

/// Normalize provider inference endpoints for the OpenAI-compatible client.
///
/// MiniMax exposes an Anthropic Messages path alongside `/v1`, Z.AI uses a
/// vendor-specific `/paas/v4` OpenAI path, and Kimi Coding needs `/coding/v1`
/// for chat completions. The source trims the URL before applying these exact
/// suffix predicates.
///
/// PARITY: agent/auxiliary_client.py lines 1010-1037.
pub fn to_openai_base_url(base_url: Option<&str>) -> String {
    let url = base_url
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .to_owned();

    if let Some(prefix) = url.strip_suffix("/anthropic") {
        if url.contains("open.bigmodel.cn") || url.contains("bigmodel") {
            return format!("{prefix}/paas/v4");
        }
        return format!("{prefix}/v1");
    }
    if url.contains("api.kimi.com") && url.ends_with("/coding") {
        return format!("{url}/v1");
    }
    url
}

/// Return whether a configured URL is the exact Anthropic API host accepted
/// by the auxiliary Anthropic client.
///
/// The source uses `urlparse`, so a bare hostname without a scheme is not
/// treated as a hostname. `base_url_hostname` supplies the URL parsing and
/// trailing-dot normalization used by the Rust utility layer.
///
/// PARITY: agent/auxiliary_client.py lines 1120-1130.
pub fn is_anthropic_compatible_host(url: &str) -> bool {
    let raw = url.trim();
    if raw.is_empty() || (!raw.contains("://") && !raw.starts_with("//")) {
        return false;
    }
    let normalized = raw
        .strip_prefix("//")
        .map_or_else(|| raw.to_owned(), |rest| format!("https://{rest}"));
    base_url_hostname(&normalized) == "api.anthropic.com"
}

/// Keepalive HTTP options passed to the eventual OpenAI-compatible SDK client.
///
/// This is the transport-neutral Rust representation of the source's httpx
/// client. `plain_scheme_mounts` means the source installs explicit HTTP and
/// HTTPS transports with the same TLS verification setting, which disables
/// httpx's ambient system-proxy lookup when no env proxy was selected.
///
/// PARITY: agent/process_bootstrap.py lines 145-213.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliaryHttpClientConfig {
    pub async_mode: bool,
    pub proxy: Option<String>,
    pub verify: AuxiliaryTlsVerify,
    pub max_keepalive_connections: usize,
    pub max_connections: usize,
    pub keepalive_expiry: Duration,
    pub connect_timeout: Duration,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Duration,
    pub pool_timeout: Duration,
    pub plain_scheme_mounts: bool,
}

/// Build the source-equivalent keepalive client configuration.
///
/// The eventual Rust HTTP implementation consumes this value to construct a
/// blocking or async client. Proxy selection is performed before deciding
/// whether the source's explicit no-proxy scheme mounts are needed.
pub fn auxiliary_http_client_config(
    base_url: Option<&str>,
    async_mode: bool,
    verify: AuxiliaryTlsVerify,
) -> AuxiliaryHttpClientConfig {
    let proxy = auxiliary_proxy_for_base_url(base_url);
    AuxiliaryHttpClientConfig {
        async_mode,
        plain_scheme_mounts: proxy.is_none(),
        proxy,
        verify,
        max_keepalive_connections: 20,
        max_connections: 100,
        keepalive_expiry: Duration::from_secs(20),
        connect_timeout: Duration::from_secs(15),
        read_timeout: None,
        write_timeout: Duration::from_secs(15),
        pool_timeout: Duration::from_secs(10),
    }
}

/// A concrete reqwest client for sync or async OpenAI-compatible requests.
///
/// The enum keeps the source's `httpx.Client`/`httpx.AsyncClient` choice
/// explicit without forcing callers to depend on an async runtime merely to
/// construct a client.
pub enum AuxiliaryHttpClient {
    Blocking(reqwest::blocking::Client),
    Async(reqwest::Client),
}

fn auxiliary_ca_certificates(
    verify: &AuxiliaryTlsVerify,
) -> Result<(Vec<reqwest::Certificate>, bool), ()> {
    match verify {
        AuxiliaryTlsVerify::CaBundle(path) => {
            let bytes = std::fs::read(path).map_err(|_| ())?;
            let certificates = reqwest::Certificate::from_pem_bundle(&bytes).map_err(|_| ())?;
            if certificates.is_empty() {
                return Err(());
            }
            // Python's ssl.create_default_context(cafile=...) uses the
            // explicit bundle as the trust store rather than adding it to
            // httpx's default certifi roots.
            Ok((certificates, true))
        }
        AuxiliaryTlsVerify::Default | AuxiliaryTlsVerify::Disabled => Ok((Vec::new(), false)),
    }
}

/// Build the concrete sync/async keepalive client and fail open on any
/// transport construction error, matching the source helper's broad
/// `except Exception: return None` boundary.
///
/// Reqwest exposes the same idle-pool and connect-timeout controls directly.
/// Its request timeout is intentionally left unset so streamed responses keep
/// the source's `read=None` behavior; the remaining write/pool timeout values
/// stay available in `AuxiliaryHttpClientConfig` for a future lower-level
/// transport implementation.
///
/// PARITY: agent/process_bootstrap.py lines 145-213.
pub fn build_auxiliary_http_client(
    config: &AuxiliaryHttpClientConfig,
) -> Option<AuxiliaryHttpClient> {
    if config.async_mode {
        build_auxiliary_async_client(config).map(AuxiliaryHttpClient::Async)
    } else {
        build_auxiliary_blocking_client(config).map(AuxiliaryHttpClient::Blocking)
    }
}

fn build_auxiliary_blocking_client(
    config: &AuxiliaryHttpClientConfig,
) -> Option<reqwest::blocking::Client> {
    let (certificates, custom_roots_only) = auxiliary_ca_certificates(&config.verify).ok()?;
    let mut builder = reqwest::blocking::Client::builder()
        .connect_timeout(config.connect_timeout)
        .pool_max_idle_per_host(config.max_keepalive_connections)
        .pool_idle_timeout(config.keepalive_expiry)
        // Reqwest otherwise reads ambient proxy variables itself. The source
        // has already applied its explicit base-URL/NO_PROXY policy.
        .no_proxy();
    if custom_roots_only {
        builder = builder.tls_built_in_root_certs(false);
    }
    for certificate in certificates {
        builder = builder.add_root_certificate(certificate);
    }
    if matches!(config.verify, AuxiliaryTlsVerify::Disabled) {
        builder = builder.danger_accept_invalid_certs(true);
    }
    if let Some(proxy) = config.proxy.as_deref() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).ok()?);
    }
    builder.build().ok()
}

fn build_auxiliary_async_client(config: &AuxiliaryHttpClientConfig) -> Option<reqwest::Client> {
    let (certificates, custom_roots_only) = auxiliary_ca_certificates(&config.verify).ok()?;
    let mut builder = reqwest::Client::builder()
        .connect_timeout(config.connect_timeout)
        .pool_max_idle_per_host(config.max_keepalive_connections)
        .pool_idle_timeout(config.keepalive_expiry)
        .no_proxy();
    if custom_roots_only {
        builder = builder.tls_built_in_root_certs(false);
    }
    for certificate in certificates {
        builder = builder.add_root_certificate(certificate);
    }
    if matches!(config.verify, AuxiliaryTlsVerify::Disabled) {
        builder = builder.danger_accept_invalid_certs(true);
    }
    if let Some(proxy) = config.proxy.as_deref() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).ok()?);
    }
    builder.build().ok()
}

/// Options passed to the eventual OpenAI-compatible auxiliary SDK client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliaryOpenAiClientConfig {
    pub api_key: String,
    pub base_url: String,
    pub max_retries: i64,
    pub http_client: Option<AuxiliaryHttpClientConfig>,
    pub default_headers: BTreeMap<String, String>,
}

/// Build the transport-independent OpenAI client options.
///
/// Hermes owns auxiliary retry and fallback policy, so the SDK retry count
/// defaults to zero while an explicit caller override wins.
///
/// The OpenAI SDK/http client construction is a future transport seam; this
/// value object preserves the source's observable option precedence now.
///
/// PARITY: agent/auxiliary_client.py lines 210-231.
pub fn openai_client_config(
    api_key: &str,
    base_url: &str,
    explicit_max_retries: Option<i64>,
) -> AuxiliaryOpenAiClientConfig {
    AuxiliaryOpenAiClientConfig {
        api_key: api_key.to_owned(),
        base_url: base_url.to_owned(),
        max_retries: explicit_max_retries.unwrap_or(0),
        http_client: None,
        default_headers: BTreeMap::new(),
    }
}

/// Build OpenAI-compatible options with the source's injected keepalive
/// client, while allowing an explicit caller-provided client to win.
///
/// This mirrors `_create_openai_client`'s `{**injected, **kwargs}` merge: the
/// default transport is added first and an explicit transport replaces it.
///
/// PARITY: agent/auxiliary_client.py lines 172-231.
pub fn openai_client_config_with_transport(
    api_key: &str,
    base_url: &str,
    explicit_max_retries: Option<i64>,
    async_mode: bool,
    verify: AuxiliaryTlsVerify,
    explicit_http_client: Option<AuxiliaryHttpClientConfig>,
) -> AuxiliaryOpenAiClientConfig {
    let mut config = openai_client_config(api_key, base_url, explicit_max_retries);
    config.http_client = explicit_http_client.or_else(|| {
        Some(auxiliary_http_client_config(
            Some(base_url),
            async_mode,
            verify,
        ))
    });
    config
}

const AUXILIARY_PROXY_ENV_KEYS: &[&str] = &[
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
];

/// Read the environment proxy used by auxiliary OpenAI-compatible clients.
///
/// The source checks uppercase names before lowercase names and normalizes the
/// first non-empty value through `normalize_proxy_url`.
///
/// PARITY: agent/process_bootstrap.py lines 112-124.
pub fn auxiliary_proxy_from_env() -> Option<String> {
    AUXILIARY_PROXY_ENV_KEYS.iter().find_map(|key| {
        let value = std::env::var(key).ok()?;
        normalize_proxy_url(Some(&value))
    })
}

fn auxiliary_no_proxy_value() -> Option<String> {
    // urllib.request.getproxies_environment() gives lowercase `_proxy`
    // variables precedence over their uppercase counterparts. An explicitly
    // empty lowercase variable removes the inherited uppercase value.
    match std::env::var("no_proxy") {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(_) => std::env::var("NO_PROXY")
            .ok()
            .filter(|value| !value.is_empty()),
    }
}

fn auxiliary_no_proxy_bypasses(host: &str) -> bool {
    let Some(no_proxy) = auxiliary_no_proxy_value() else {
        return false;
    };
    if no_proxy == "*" {
        return true;
    }

    let host = host.to_lowercase();
    no_proxy.split(',').any(|raw_name| {
        let name = raw_name.trim().trim_start_matches('.').to_lowercase();
        if name.is_empty() {
            return false;
        }
        host == name || host.ends_with(&format!(".{name}"))
    })
}

/// Return the environment proxy unless `NO_PROXY` excludes the base URL.
///
/// A missing or malformed hostname fails open to the configured proxy, just
/// like the source's `proxy_bypass_environment` call.
///
/// PARITY: agent/process_bootstrap.py lines 126-145.
pub fn auxiliary_proxy_for_base_url(base_url: Option<&str>) -> Option<String> {
    let proxy = auxiliary_proxy_from_env();
    let Some(base_url) = base_url.filter(|value| !value.is_empty()) else {
        return proxy;
    };
    let host = base_url_hostname(base_url);
    if host.is_empty() {
        return proxy;
    }
    if auxiliary_no_proxy_bypasses(&host) {
        return None;
    }
    proxy
}

/// Input shape for the source's dynamically typed `ssl_verify` setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliarySslVerifySetting {
    Boolean(bool),
    Text(String),
}

/// Transport-independent representation of the `httpx verify` choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryTlsVerify {
    Default,
    Disabled,
    CaBundle(String),
}

fn auxiliary_ssl_verification_disabled(setting: Option<&AuxiliarySslVerifySetting>) -> bool {
    match setting {
        Some(AuxiliarySslVerifySetting::Boolean(false)) => true,
        Some(AuxiliarySslVerifySetting::Text(value)) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        ),
        _ => false,
    }
}

fn auxiliary_expand_user(path: &str) -> String {
    if path == "~" || path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home = home.to_string_lossy();
            return if path == "~" {
                home.into_owned()
            } else {
                format!("{home}/{}", &path[2..])
            };
        }
    }
    path.to_owned()
}

/// The operator-facing warning `resolve_httpx_verify` emits.
///
/// PARITY: the two `logger.warning` calls in `agent/ssl_verify.py`
/// (lines 40-44 and 58-61). Upstream logs them from inside the resolver; the
/// Rust port exposes them as a value so the text is testable, and
/// [`resolve_auxiliary_tls_verify`] logs the same string through `log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxiliaryTlsVerifyWarning {
    /// Verification was disabled explicitly. `base_url` is the endpoint named
    /// in the message; `None` reproduces the source's
    /// `"a custom provider endpoint"` placeholder for callers without a URL.
    Insecure { base_url: Option<String> },
    /// The selected CA bundle path does not exist, so the default certificate
    /// store is used instead. The path is reported before `~` expansion, as the
    /// source reports the configured value.
    MissingCaBundle { configured: String },
}

impl AuxiliaryTlsVerifyWarning {
    /// The exact message text the source logs.
    pub fn text(&self) -> String {
        match self {
            Self::Insecure { base_url } => format!(
                "TLS certificate verification DISABLED (ssl_verify: false) for {} — \
                 this is intended for local development only and is unsafe on any \
                 network you do not fully control.",
                base_url
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("a custom provider endpoint")
            ),
            Self::MissingCaBundle { configured } => format!(
                "CA bundle path does not exist: {configured} — falling back to \
                 default certificates"
            ),
        }
    }
}

/// The first non-empty CA bundle source in the env precedence chain, without
/// the existence check.
///
/// PARITY: the `effective_ca` `or` chain (`agent/ssl_verify.py` lines 46-52).
/// The source's `or` chain means a non-empty explicit path wins even when it is
/// stale — a missing explicit path therefore falls back to the default
/// certificates rather than silently trying a lower-priority env variable.
fn auxiliary_configured_ca_bundle(ca_bundle: Option<&str>) -> Option<String> {
    let env_value = |key: &str| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    ca_bundle
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| env_value("HERMES_CA_BUNDLE"))
        .or_else(|| env_value("SSL_CERT_FILE"))
        .or_else(|| env_value("REQUESTS_CA_BUNDLE"))
        .or_else(|| env_value("CURL_CA_BUNDLE"))
}

/// Resolve the source's `httpx verify` setting without binding the Rust port
/// to a particular HTTP client implementation.
///
/// The caller supplies the already-resolved provider TLS fields. A missing or
/// invalid CA path fails open to the HTTP client's default certificate store,
/// and `base_url` is used only for the insecure-mode warning message, exactly as
/// the source documents.
pub fn resolve_auxiliary_tls_verify(
    ca_bundle: Option<&str>,
    ssl_verify: Option<&AuxiliarySslVerifySetting>,
    base_url: Option<&str>,
) -> AuxiliaryTlsVerify {
    let (verify, warning) = auxiliary_tls_verify_resolution(ca_bundle, ssl_verify, base_url);
    if let Some(warning) = &warning {
        log::warn!("{}", warning.text());
    }
    verify
}

/// The resolved setting together with the warning the source would log.
///
/// PARITY: `resolve_httpx_verify` (`agent/ssl_verify.py` lines 23-65) in its
/// pure form: insecure coercion first, then the CA bundle precedence chain and
/// its existence check, then the default.
pub fn auxiliary_tls_verify_resolution(
    ca_bundle: Option<&str>,
    ssl_verify: Option<&AuxiliarySslVerifySetting>,
    base_url: Option<&str>,
) -> (AuxiliaryTlsVerify, Option<AuxiliaryTlsVerifyWarning>) {
    if auxiliary_ssl_verification_disabled(ssl_verify) {
        return (
            AuxiliaryTlsVerify::Disabled,
            Some(AuxiliaryTlsVerifyWarning::Insecure {
                base_url: base_url.map(str::to_string),
            }),
        );
    }
    match auxiliary_configured_ca_bundle(ca_bundle) {
        None => (AuxiliaryTlsVerify::Default, None),
        Some(configured) => {
            let expanded = auxiliary_expand_user(&configured);
            if Path::new(&expanded).is_file() {
                (AuxiliaryTlsVerify::CaBundle(expanded), None)
            } else {
                (
                    AuxiliaryTlsVerify::Default,
                    Some(AuxiliaryTlsVerifyWarning::MissingCaBundle { configured }),
                )
            }
        }
    }
}

/// Build the first-party headers required by the Codex OAuth endpoint.
///
/// Invalid or non-JWT tokens retain the fixed headers and simply omit the
/// optional account identifier, preserving the source's auth-error path.
///
/// PARITY: agent/auxiliary_client.py lines 971-1002.
pub fn codex_cloudflare_headers(access_token: &str) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::from([
        (
            "User-Agent".to_owned(),
            "codex_cli_rs/0.0.0 (Hermes Agent)".to_owned(),
        ),
        ("originator".to_owned(), "codex_cli_rs".to_owned()),
    ]);
    if access_token.trim().is_empty() {
        return headers;
    }

    let Some(payload_part) = access_token.split('.').nth(1) else {
        return headers;
    };
    let mut encoded = payload_part.to_owned();
    encoded.push_str(&"=".repeat((4 - encoded.len() % 4) % 4));
    let Ok(decoded) = URL_SAFE.decode(encoded.as_bytes()) else {
        return headers;
    };
    let Ok(claims) = serde_json::from_slice::<Value>(&decoded) else {
        return headers;
    };
    let account_id = claims
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object)
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    if let Some(account_id) = account_id {
        headers.insert("ChatGPT-Account-ID".to_owned(), account_id.to_owned());
    }
    headers
}

fn codex_access_token_expired(access_token: &str, now_epoch_seconds: u64) -> bool {
    let Some(payload_part) = access_token.split('.').nth(1) else {
        return false;
    };
    let mut encoded = payload_part.to_owned();
    encoded.push_str(&"=".repeat((4 - encoded.len() % 4) % 4));
    let Ok(decoded) = URL_SAFE.decode(encoded.as_bytes()) else {
        return false;
    };
    let Ok(claims) = serde_json::from_slice::<Value>(&decoded) else {
        return false;
    };
    let Some(exp_value) = claims.get("exp") else {
        return false;
    };
    let exp = match exp_value {
        Value::Number(value) => value.as_f64().unwrap_or(0.0),
        // Python treats bool as an int during the `time.time() > exp`
        // comparison, while false is skipped by the preceding truthiness
        // check.
        Value::Bool(value) => u8::from(*value) as f64,
        _ => return false,
    };
    exp != 0.0 && (now_epoch_seconds as f64) > exp
}

/// Select a usable Codex access token from an explicit pool projection or the
/// already-resolved Hermes auth-store token object.
///
/// The pool flag and auth-store value are explicit adapters for the source's
/// credential-pool and `hermes_cli.auth._read_codex_tokens()` lookups. A
/// malformed/non-JWT token is retained, while a decoded expired JWT is
/// rejected so provider fallback can continue.
///
/// PARITY: agent/auxiliary_client.py lines 2279-2317.
pub fn read_codex_access_token(
    pool_present: bool,
    pool_entry: Option<&AuxiliaryPoolEntry>,
    auth_tokens: Option<&Value>,
    now_epoch_seconds: u64,
) -> Option<String> {
    if pool_present {
        let pool_token = pool_runtime_api_key(pool_entry);
        if !pool_token.is_empty() {
            return Some(pool_token);
        }
    }

    let access_token = auth_tokens
        .and_then(|value| value.get("tokens"))
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())?;
    if codex_access_token_expired(access_token, now_epoch_seconds) {
        return None;
    }
    Some(access_token.to_owned())
}

/// Normalize an auxiliary provider name and its source aliases.
///
/// main_provider is the explicit adapter for the source's lazy
/// _read_main_provider() call when provider == main.
///
/// PARITY: agent/auxiliary_client.py lines 487-549.
pub fn normalize_aux_provider(provider: Option<&str>, main_provider: Option<&str>) -> String {
    let mut normalized = provider.unwrap_or("auto").trim().to_ascii_lowercase();
    if let Some(suffix) = normalized.strip_prefix("custom:") {
        let suffix = suffix.trim();
        if suffix.is_empty() {
            return "custom".into();
        }
        normalized = suffix.into();
    }
    if normalized == "codex" {
        return "openai-codex".into();
    }
    if normalized == "main" {
        let main = main_provider
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !main.is_empty() && main != "auto" && main != "main" {
            normalized = main;
        } else {
            return "custom".into();
        }
    }
    PROVIDER_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == normalized).then_some(*canonical))
        .unwrap_or(&normalized)
        .to_string()
}

/// Return the OpenAI-compatible output-token keyword for an auxiliary call.
///
/// The booleans make the source's process/config lookups explicit: true means
/// the scoped OpenRouter key or Nous auth is present.
///
/// PARITY: agent/auxiliary_client.py lines 6935-6961.
pub fn auxiliary_max_tokens_param(
    value: i64,
    model: Option<&str>,
    custom_base_url: Option<&str>,
    openrouter_key_present: bool,
    nous_auth_present: bool,
) -> Value {
    let custom_host = base_url_hostname(custom_base_url.unwrap_or(""));
    if !openrouter_key_present
        && !nous_auth_present
        && (custom_host == "api.openai.com"
            || custom_host == "api.githubcopilot.com"
            || custom_host.ends_with(".githubcopilot.com"))
    {
        return json!({"max_completion_tokens": value});
    }
    if model_forces_max_completion_tokens(model.unwrap_or("")) {
        return json!({"max_completion_tokens": value});
    }
    json!({"max_tokens": value})
}
fn lower_message(error: &AuxiliaryError) -> String {
    error.message.to_ascii_lowercase()
}

fn contains_any(message: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| message.contains(keyword))
}

/// Detect payment, billing, and exhausted-quota errors.
///
/// PARITY: agent/auxiliary_client.py lines 3722-3761.
pub fn is_payment_error(error: &AuxiliaryError) -> bool {
    if error.status_code == Some(402) {
        return true;
    }
    if !matches!(error.status_code, None | Some(402 | 403 | 404 | 429)) {
        return false;
    }
    contains_any(
        &lower_message(error),
        &[
            "credits",
            "insufficient funds",
            "can only afford",
            "billing",
            "payment required",
            "out of funds",
            "run out of funds",
            "balance_depleted",
            "no usable credits",
            "model_not_supported_on_free_tier",
            "not available on the free tier",
            "requires a subscription",
            "upgrade for access",
            "upgrade for higher limits",
            "reached your session usage limit",
            "quota exceeded",
            "quota_exceeded",
            "too many tokens per day",
            "daily limit",
            "tokens per day",
            "daily quota",
            "resource exhausted",
            "weekly usage limit",
            "weekly limit",
        ],
    )
}

/// Detect provider rate-limit errors while excluding billing 429s.
///
/// PARITY: agent/auxiliary_client.py lines 3776-3810.
pub fn is_rate_limit_error(error: &AuxiliaryError) -> bool {
    if error.type_name == "RateLimitError" {
        return true;
    }
    if error.status_code != Some(429) {
        return false;
    }
    let message = lower_message(error);
    if contains_any(
        &message,
        &[
            "rate limit",
            "rate_limit",
            "too many requests",
            "try again",
            "retry after",
            "resets in",
        ],
    ) {
        return true;
    }
    !contains_any(
        &message,
        &[
            "credits",
            "insufficient funds",
            "billing",
            "payment required",
            "can only afford",
            "out of funds",
            "run out of funds",
            "balance_depleted",
            "no usable credits",
            "model_not_supported_on_free_tier",
            "not available on the free tier",
        ],
    )
}

/// Detect stale or invalid model identifiers.
///
/// PARITY: agent/auxiliary_client.py lines 3980-4017.
pub fn is_model_not_found_error(error: &AuxiliaryError) -> bool {
    let message = lower_message(error);
    if contains_any(
        &message,
        &[
            "credits",
            "insufficient funds",
            "billing",
            "out of funds",
            "balance_depleted",
            "no usable credits",
            "free tier",
            "free-tier",
            "not available on the free tier",
        ],
    ) {
        return false;
    }
    if !matches!(error.status_code, None | Some(400 | 404)) {
        return false;
    }
    contains_any(
        &message,
        &[
            "model does not exist",
            "does not exist in our configuration",
            "openrouter catalog",
            "is not a valid model",
            "no such model",
            "model not found",
            "the model `",
            "model_not_found",
            "unknown model",
        ],
    )
}

/// Detect a valid model that the current route/account cannot serve.
///
/// PARITY: agent/auxiliary_client.py lines 4020-4068.
pub fn is_model_incompatible_error(error: &AuxiliaryError) -> bool {
    if !matches!(error.status_code, None | Some(400)) {
        return false;
    }
    let message = lower_message(error);
    if is_model_not_found_error(error)
        || contains_any(
            &message,
            &[
                "credits",
                "insufficient funds",
                "billing",
                "out of funds",
                "balance_depleted",
                "no usable credits",
                "payment required",
                "free tier",
                "free-tier",
                "not available on the free tier",
                "model_not_supported_on_free_tier",
                "quota",
            ],
        )
    {
        return false;
    }
    contains_any(
        &message,
        &[
            "is not supported when using",
            "model is not supported",
            "not supported with this",
            "not supported for this account",
            "model_not_supported",
            "does not support this model",
            "unsupported model",
        ],
    )
}

// ── Client-level headers and Portal extra_body ─────────────────────────────
//
// Upstream keeps these builders next to the credential-pool and client
// construction code they feed. The request/transport lifecycle above them is
// still unported, so each function here is a pure seam over explicit inputs.

/// Python truthiness for the `openrouter.response_cache` leaf.
///
/// PARITY: `or_config.get("response_cache", False)` (upstream line 878).
fn response_cache_enabled(section: &serde_json::Map<String, Value>) -> bool {
    json_truthy(section.get("response_cache"))
}

/// Build OpenRouter attribution and response-cache headers.
///
/// PARITY: `build_or_headers` (upstream `agent/auxiliary_client.py` lines
/// 848-898): the base attribution headers are always present; the cache
/// header is gated by env-over-config precedence; and the TTL header is
/// emitted only for integers in `[1, 86400]`.
///
/// `or_config` is the `openrouter` *section*, not the whole config. When it is
/// `None` the source falls back to `load_config_readonly().get("openrouter")`,
/// which merges `DEFAULT_CONFIG` — so a missing config file still enables
/// caching with the default TTL, mirrored here through
/// [`crate::config::openrouter_defaults`].
///
/// Divergence kept deliberately: Python's `isinstance(ttl, (int, float))` also
/// accepts `True`/`False` (bool is an `int` subclass), so a YAML `true` TTL
/// emits `"1"` upstream; that quirk is reproduced rather than "fixed".
pub fn build_or_headers(
    or_config: Option<&serde_json::Map<String, Value>>,
) -> BTreeMap<String, String> {
    // PARITY: `_OR_HEADERS_BASE` (upstream lines 797-801).
    let mut headers = BTreeMap::new();
    headers.insert(
        "HTTP-Referer".into(),
        "https://hermes-agent.nousresearch.com".into(),
    );
    headers.insert("X-Title".into(), "Hermes Agent".into());
    headers.insert(
        "X-OpenRouter-Categories".into(),
        "productivity,cli-agent".into(),
    );

    let section = match or_config {
        Some(section) => section.clone(),
        None => {
            let snapshot = load_merged_config_snapshot(&openrouter_defaults());
            match snapshot.pool_config.get("openrouter") {
                Some(Value::Object(section)) => section.clone(),
                // `.get("openrouter", {})`: any non-mapping shape behaves as {}.
                _ => serde_json::Map::new(),
            }
        }
    };

    // PARITY: env var overrides config (upstream lines 872-878); only the
    // exact truthy spellings enable caching.
    let env_cache = std::env::var("HERMES_OPENROUTER_CACHE")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let cache_enabled = if env_cache.is_empty() {
        response_cache_enabled(&section)
    } else {
        matches!(env_cache.as_str(), "1" | "true" | "yes" | "on")
    };
    if !cache_enabled {
        return headers;
    }
    headers.insert("X-OpenRouter-Cache".into(), "true".into());

    // PARITY: TTL precedence and bounds (upstream lines 885-896).
    let env_ttl = std::env::var("HERMES_OPENROUTER_CACHE_TTL")
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    if !env_ttl.is_empty() {
        // `str.isdigit()`: ASCII digits only, so `-1`, `12.5`, and `abc` drop
        // the header while the cache header survives.
        if let Some(digits) = env_ttl
            .chars()
            .all(|ch| ch.is_ascii_digit())
            .then_some(env_ttl.as_str())
        {
            if let Ok(ttl) = digits.parse::<i64>() {
                if (1..=86400).contains(&ttl) {
                    headers.insert("X-OpenRouter-Cache-TTL".into(), ttl.to_string());
                }
            }
        }
    } else {
        let ttl = match section.get("response_cache_ttl") {
            Some(Value::Bool(value)) => Some(i64::from(*value)),
            Some(Value::Number(value)) => value.as_i64().or_else(|| {
                value
                    .as_f64()
                    .filter(|float| float.is_finite())
                    .map(|float| float as i64)
            }),
            _ => None,
        };
        if let Some(ttl) = ttl {
            if (1..=86400).contains(&ttl) {
                headers.insert("X-OpenRouter-Cache-TTL".into(), ttl.to_string());
            }
        }
    }
    headers
}

/// PARITY: `str(value)` applied to a header leaf (upstream line 845). Booleans
/// render with Python casing; the degenerate container shapes render as JSON
/// rather than Python's `repr` (single quotes), which no upstream test pins.
fn python_header_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => {
            if *value {
                "True".into()
            } else {
                "False".into()
            }
        }
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
        Value::Null => "None".into(),
    }
}

/// Overlay user-configured request headers onto resolved client headers.
///
/// PARITY: `_apply_user_default_headers` (upstream `agent/auxiliary_client.py`
/// lines 807-846): user values win over provider/SDK defaults so a `custom`
/// endpoint behind a gateway that rejects the SDK's identifying headers works
/// for auxiliary calls too (#40033). `model.extra_headers` is the accepted
/// alias; when both are set they merge with `extra_headers` winning. Only an
/// explicit `null` value is skipped — keys are used verbatim (Python does not
/// trim them) — and non-string scalars are stringified.
///
/// `config` is the FULL merged config. `None` mirrors the source's internal
/// `load_config()` read; a config-load failure is fail-open upstream and is
/// represented by an absent `model` section here.
pub fn apply_user_default_headers(
    headers: &mut BTreeMap<String, String>,
    config: Option<&serde_json::Map<String, Value>>,
) {
    let resolved;
    let config = match config {
        Some(config) => config,
        None => {
            resolved = load_merged_config_snapshot(&openrouter_defaults()).pool_config;
            &resolved
        }
    };
    let user_headers = match cfg_get(config, &["model", "default_headers"], Value::Null) {
        Value::Object(map) => Some(map),
        _ => None,
    };
    // PARITY: alias merge (upstream lines 829-835): a non-empty
    // `model.extra_headers` overrides same-named `default_headers` entries.
    let alias = cfg_get(config, &["model", "extra_headers"], Value::Null);
    let mut user_headers = match alias {
        Value::Object(alias) if !alias.is_empty() => {
            let mut merged = user_headers.unwrap_or_default();
            for (key, value) in alias {
                merged.insert(key, value);
            }
            Some(merged)
        }
        _ => user_headers,
    };
    // PARITY: `if not isinstance(user_headers, dict) or not user_headers`
    // (upstream lines 838-839): nothing configured means no allocation.
    let Some(raw) = user_headers.take().filter(|map| !map.is_empty()) else {
        return;
    };
    for (key, value) in raw {
        if value.is_null() {
            continue;
        }
        headers.insert(key, python_header_value(&value));
    }
}

/// Return NVIDIA NIM cloud attribution headers for build.nvidia.com traffic.
///
/// PARITY: `build_nvidia_nim_headers` (upstream lines 902-911): host-gated
/// because the nvidia provider also serves local/on-prem NIM endpoints via
/// `NVIDIA_BASE_URL`.
pub fn build_nvidia_nim_headers(base_url: Option<&str>) -> BTreeMap<String, String> {
    if base_url_host_matches(base_url.unwrap_or_default(), "integrate.api.nvidia.com") {
        let mut headers = BTreeMap::new();
        headers.insert("X-BILLING-INVOKE-ORIGIN".into(), "HermesAgent".into());
        return headers;
    }
    BTreeMap::new()
}

/// Build the standard headers for Copilot API requests.
///
/// PARITY: `copilot_request_headers` (upstream `hermes_cli/copilot_auth.py`
/// lines 674-693): replicates the opencode/Copilot CLI header set. OAuth token
/// attachment and the `hermes_cli.models.copilot_default_headers` import
/// fallback (lines 3430-3445) live above this pure builder.
pub fn copilot_request_headers(is_agent_turn: bool, is_vision: bool) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    headers.insert("Editor-Version".into(), "vscode/1.104.1".into());
    headers.insert("User-Agent".into(), "HermesAgent/1.0".into());
    headers.insert("Copilot-Integration-Id".into(), "vscode-chat".into());
    headers.insert("Openai-Intent".into(), "conversation-edits".into());
    headers.insert(
        "x-initiator".into(),
        if is_agent_turn { "agent" } else { "user" }.into(),
    );
    if is_vision {
        headers.insert("Copilot-Vision-Request".into(), "true".into());
    }
    headers
}

/// Resolve host-gated provider headers for an auxiliary OpenAI-compatible
/// client.
///
/// PARITY: the credential-pool chain (upstream lines 2369-2386) and the
/// custom-endpoint chain (lines 6044-6062) apply the same
/// kimi/copilot/nvidia gates in the same order and the same
/// `get_provider_profile(...).default_headers` fallback, so one pure function
/// covers both call sites. Differences that stay with the callers:
///   * the pool site reaches Copilot through
///     `hermes_cli.models.copilot_default_headers()`, which never forwards a
///     vision flag — pass `is_vision: false` to reproduce it;
///   * an empty base URL carries no headers, matching the source's
///     `if custom_base and custom_key` guard;
///   * the async-conversion chain (lines 5654-5684) additionally gates on
///     `openrouter.ai` and `x.ai` and infers the provider from the URL; that
///     route is covered by [`openrouter_cache_headers`] plus the future
///     xAI header seam.
pub fn resolve_provider_default_headers(
    provider: Option<&str>,
    base_url: Option<&str>,
    is_vision: bool,
) -> BTreeMap<String, String> {
    let base = base_url.unwrap_or_default().trim();
    if base.is_empty() {
        return BTreeMap::new();
    }
    if base_url_host_matches(base, "api.kimi.com") {
        let mut headers = BTreeMap::new();
        headers.insert("User-Agent".into(), "claude-code/0.1.0".into());
        headers
    } else if base_url_host_matches(base, "githubcopilot.com") {
        copilot_request_headers(true, is_vision)
    } else if base_url_host_matches(base, "integrate.api.nvidia.com") {
        build_nvidia_nim_headers(Some(base))
    } else {
        provider
            .and_then(get_provider_profile)
            .map(|profile| profile.default_headers)
            .unwrap_or_default()
    }
}

/// Return OpenRouter attribution and response-cache headers for an auxiliary
/// client.
///
/// PARITY: the two OpenRouter header sites in the source: the
/// `provider == "openrouter"` route builds its client with
/// `build_or_headers()` regardless of the pool base URL (upstream
/// `_try_openrouter`, lines 2479-2515, reached from line 5910), and the
/// sync-to-async conversion applies them whenever the client host is
/// `openrouter.ai` (lines 5654-5655). This helper is the union of those two
/// gates: an openrouter route, or an openrouter host.
///
/// `or_config` is the `openrouter` section (upstream's `build_or_headers`
/// argument); `None` keeps the source's own `load_config_readonly()` read.
pub fn openrouter_cache_headers(
    provider: Option<&str>,
    base_url: Option<&str>,
    or_config: Option<&serde_json::Map<String, Value>>,
) -> BTreeMap<String, String> {
    let provider = provider.unwrap_or_default().trim().to_ascii_lowercase();
    let base = base_url.unwrap_or_default();
    if provider != "openrouter" && !base_url_host_matches(base, "openrouter.ai") {
        return BTreeMap::new();
    }
    build_or_headers(or_config)
}

/// Compose the full client-level default headers for an auxiliary
/// OpenAI-compatible client.
///
/// PARITY: source header assembly order — host-gated provider headers first
/// (upstream lines 2369-2386 and 6044-6062), OpenRouter cache headers merged
/// over them (lines 2502-2514 and 5654-5655), and the user's
/// `model.default_headers` overlay winning last (lines 2386-2389, 6063-6066,
/// and 5684-5687).
pub fn auxiliary_default_headers(
    provider: Option<&str>,
    base_url: Option<&str>,
    is_vision: bool,
    config: Option<&serde_json::Map<String, Value>>,
) -> BTreeMap<String, String> {
    // `.get("openrouter", {})`: a supplied config that says nothing about
    // OpenRouter contributes an empty section, not the disk fallback.
    let or_section = config.map(
        |config| match cfg_get(config, &["openrouter"], Value::Null) {
            Value::Object(section) => section,
            _ => serde_json::Map::new(),
        },
    );
    let mut headers = resolve_provider_default_headers(provider, base_url, is_vision);
    headers.extend(openrouter_cache_headers(
        provider,
        base_url,
        or_section.as_ref(),
    ));
    apply_user_default_headers(&mut headers, config);
    headers
}

/// Return a fresh Nous Portal `extra_body` document.
///
/// PARITY: `_nous_extra_body` (upstream lines 935-941): computed at call time
/// so a changed `hermes_cli.__version__` is reflected without restarting
/// long-running processes. Upstream also keeps a deprecated module-level
/// `NOUS_EXTRA_BODY` snapshot of this value (lines 944-948); consumers are
/// expected to call the helper, so the snapshot is not reproduced.
pub fn nous_extra_body() -> serde_json::Map<String, Value> {
    let mut body = serde_json::Map::new();
    body.insert(
        "tags".to_string(),
        Value::Array(
            crate::portal_tags::nous_portal_tags(None)
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    body
}

/// Return extra_body kwargs for auxiliary API calls.
///
/// PARITY: `get_auxiliary_extra_body` (upstream lines 6926-6932): Nous Portal
/// product tags when the auxiliary client is backed by Nous Portal, empty
/// otherwise. `auxiliary_is_nous` is the module global the source sets at
/// resolve time; it is an explicit argument here.
pub fn get_auxiliary_extra_body(auxiliary_is_nous: bool) -> serde_json::Map<String, Value> {
    if auxiliary_is_nous {
        nous_extra_body()
    } else {
        serde_json::Map::new()
    }
}

/// Return the Nous Portal fallback `tags`/`session_id` entries for transport
/// client kwargs.
///
/// PARITY: the portal fallback in `_create_transport_client` (upstream lines
/// 8068-8086): only for nous spellings, and only for a key the profile merge
/// did not already supply. The sticky `session_id` comes from the ambient
/// conversation context — tags alone are not enough on `/v1/messages`, where
/// the sticky key keeps auxiliary compression/title/vision calls on the same
/// upstream instance as the main turn (cache warmth).
pub fn nous_portal_fallback_extra(
    provider: Option<&str>,
    has_tags: bool,
    has_session_id: bool,
) -> serde_json::Map<String, Value> {
    let provider = provider.unwrap_or_default().trim().to_ascii_lowercase();
    if !matches!(provider.as_str(), "nous" | "nous-portal" | "nousresearch") {
        return serde_json::Map::new();
    }
    let mut extra = serde_json::Map::new();
    if !has_tags {
        extra.insert(
            "tags".into(),
            Value::Array(
                crate::portal_tags::nous_portal_tags(None)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if !has_session_id {
        // PARITY: `if sticky_key:` — an empty ambient id contributes nothing.
        if let Some(sticky) = get_conversation_context().filter(|value| !value.is_empty()) {
            extra.insert("session_id".into(), Value::String(sticky));
        }
    }
    extra
}
