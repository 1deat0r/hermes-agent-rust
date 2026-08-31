//! Provider-agnostic billing/credit recovery links.
//!
//! PARITY: `agent/billing_links.py` @ b9aa928 (whole module).
//!
//! Maps a billing-classified failure onto a recovery link + label.
//! *Detection* is not done here — that is `agent.error_classifier`
//! (`FailoverReason.billing`), the single source of truth for "credit wall
//! vs. rate limit / auth / transport". The resulting [`BillingBlock`] rides
//! the turn result and the gateway `message.complete` event so every
//! surface (CLI, TUI, desktop) renders one structured signal instead of
//! re-parsing error text.
//!
//! TRANSLATION NOTE: upstream's `_nous_billing_url` best-effort imports
//! `hermes_cli.nous_account.nous_portal_billing_url` (fail-open to the
//! constant fallback). The hermes_cli crate sits above agent in the Rust
//! layering, so the constant fallback is the only path here — documented
//! divergence.

use serde_json::{json, Value};

use hermes_utils::base_url_host_matches;

/// Structured billing-wall descriptor shared across every surface.
///
/// `is_nous` is the routing bit: Nous has a first-class in-app billing
/// surface (desktop Settings → Billing, TUI/CLI `/topup`), so surfaces
/// prefer that over `billing_url`; third-party providers have no in-app
/// flow, so `billing_url` is the deep link the user actually needs.
///
/// PARITY: `BillingBlock` (upstream lines 23-40).
#[derive(Debug, Clone, PartialEq)]
pub struct BillingBlock {
    pub provider: String,
    pub provider_label: String,
    pub model: String,
    pub billing_url: Option<String>,
    pub is_nous: bool,
    pub message: String,
}

impl BillingBlock {
    /// PARITY: `to_dict` (upstream `asdict`).
    pub fn to_dict(&self) -> Value {
        json!({
            "provider": self.provider,
            "provider_label": self.provider_label,
            "model": self.model,
            "billing_url": self.billing_url,
            "is_nous": self.is_nous,
            "message": self.message,
        })
    }
}

#[derive(Debug, Clone)]
struct Provider {
    label: &'static str,
    url: &'static str,
    slugs: &'static [&'static str],
    hosts: &'static [&'static str],
}

/// Single source of truth: internal slug(s) + base_url host(s) → billing
/// page. Curated "add credits / manage billing" landing pages, not
/// marketing homes. Hosts back the OpenAI-compatible fallback where the
/// slug is a generic bucket but base_url reveals the real upstream. An
/// unknown provider degrades to a readable label with no invented URL.
///
/// PARITY: `_PROVIDERS` (upstream lines 66-82).
const PROVIDERS: [Provider; 14] = [
    Provider {
        label: "OpenAI",
        url: "https://platform.openai.com/settings/organization/billing",
        slugs: &["openai"],
        hosts: &["api.openai.com"],
    },
    Provider {
        label: "Anthropic",
        url: "https://console.anthropic.com/settings/billing",
        slugs: &["anthropic"],
        hosts: &["api.anthropic.com"],
    },
    Provider {
        label: "OpenRouter",
        url: "https://openrouter.ai/settings/credits",
        slugs: &["openrouter"],
        hosts: &["openrouter.ai"],
    },
    Provider {
        label: "xAI",
        url: "https://console.x.ai/team/default/billing",
        slugs: &["xai", "xai-oauth"],
        hosts: &["api.x.ai"],
    },
    Provider {
        label: "DeepSeek",
        url: "https://platform.deepseek.com/top_up",
        slugs: &["deepseek"],
        hosts: &["api.deepseek.com"],
    },
    Provider {
        label: "Groq",
        url: "https://console.groq.com/settings/billing",
        slugs: &["groq"],
        hosts: &["api.groq.com"],
    },
    Provider {
        label: "Mistral",
        url: "https://console.mistral.ai/billing",
        slugs: &["mistral"],
        hosts: &["api.mistral.ai"],
    },
    Provider {
        label: "Together AI",
        url: "https://api.together.ai/settings/billing",
        slugs: &["together"],
        hosts: &["api.together.ai", "api.together.xyz"],
    },
    Provider {
        label: "Fireworks AI",
        url: "https://fireworks.ai/account/billing",
        slugs: &["fireworks"],
        hosts: &["fireworks.ai"],
    },
    Provider {
        label: "Perplexity",
        url: "https://www.perplexity.ai/settings/api",
        slugs: &["perplexity"],
        hosts: &["perplexity.ai"],
    },
    Provider {
        label: "Google AI",
        url: "https://aistudio.google.com/app/billing",
        slugs: &["google", "gemini"],
        hosts: &["generativelanguage.googleapis.com"],
    },
    Provider {
        label: "Cohere",
        url: "https://dashboard.cohere.com/billing",
        slugs: &["cohere"],
        hosts: &[],
    },
    Provider {
        label: "Moonshot AI",
        url: "https://platform.moonshot.ai/console/pay",
        slugs: &["moonshot"],
        hosts: &[],
    },
    Provider {
        label: "NVIDIA",
        url: "https://build.nvidia.com/settings/billing",
        slugs: &["nvidia"],
        hosts: &[],
    },
];

/// True when the failing route is the Nous-managed inference gateway.
///
/// PARITY: `is_nous_inference_route` (upstream lines 85-89).
pub fn is_nous_inference_route(provider: &str, base_url: &str) -> bool {
    if provider.trim().to_lowercase() == "nous" {
        return true;
    }
    base_url_host_matches(base_url, "inference-api.nousresearch.com")
}

/// Best-effort Nous portal billing URL. Nous prefers the in-app flow; this
/// constant is the text-surface fallback (the hermes_cli
/// `nous_portal_billing_url` lookup is PENDING with that surface — the
/// upstream import fails open to this same constant).
///
/// PARITY: `_nous_billing_url` fallback arm (upstream lines 96-103).
fn nous_billing_url() -> String {
    "https://portal.nousresearch.com/billing".to_string()
}

/// Resolve `(label, url)`: exact slug → base_url host → readable-label
/// fallback.
///
/// PARITY: `_resolve_provider_link` (upstream lines 106-119).
fn resolve_provider_link(slug: &str, base_url: &str) -> (String, Option<String>) {
    // Pass 1: exact slug (the `_BY_SLUG` dict lookup).
    for p in &PROVIDERS {
        if p.slugs.contains(&slug) {
            return (p.label.to_string(), Some(p.url.to_string()));
        }
    }
    // Pass 2: base_url host match.
    for p in &PROVIDERS {
        for host in p.hosts {
            if base_url_host_matches(base_url, host) {
                return (p.label.to_string(), Some(p.url.to_string()));
            }
        }
    }
    let label = slug.replace('_', " ").replace('-', " ").trim().to_string();
    let label = if label.is_empty() {
        "your provider".to_string()
    } else {
        title_case(&label)
    };
    (label, None)
}

/// Python `str.title()` over an ASCII-ish lowercase string.
fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build the billing descriptor for a billing-classified failure.
///
/// `message` is the guidance already assembled by the agent loop, carried
/// through unchanged so every surface shows identical copy.
///
/// PARITY: `build_billing_block` (upstream lines 122-141).
pub fn build_billing_block(
    provider: &str,
    base_url: &str,
    model: &str,
    message: &str,
) -> BillingBlock {
    let slug = provider.trim().to_lowercase();
    let model = model.trim().to_string();

    if is_nous_inference_route(&slug, base_url) {
        return BillingBlock {
            provider: if slug.is_empty() {
                "nous".to_string()
            } else {
                slug
            },
            provider_label: "Nous Portal".to_string(),
            model,
            billing_url: Some(nous_billing_url()),
            is_nous: true,
            message: message.to_string(),
        };
    }

    let (label, url) = resolve_provider_link(&slug, base_url);
    BillingBlock {
        provider: slug,
        provider_label: label,
        model,
        billing_url: url,
        is_nous: false,
        message: message.to_string(),
    }
}
