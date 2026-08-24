//! Dependency-safe parity helpers from `agent/auxiliary_client.py`.
//!
//! This module starts with pure routing predicates and wire parameter helpers.
//! Client construction, credential pools, async transport, cancellation, and
//! provider fallback chains remain higher-layer sections of the 10,044-line
//! upstream module.

use hermes_utils::{base_url_hostname, model_forces_max_completion_tokens};
use serde_json::{json, Value};
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
