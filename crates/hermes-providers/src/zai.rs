//! Z.AI / GLM provider profile.
//!
//! PARITY: plugins/model-providers/zai/__init__.py @ b9aa928.

use crate::base::ProviderProfile;

/// A Z.AI endpoint and its ordered candidate models.
///
/// The four entries and model order mirror `hermes_cli/auth.py` lines
/// 685–691 at upstream commit `b9aa928`.  Endpoint order is significant:
/// [`choose_zai_endpoint`] always prefers earlier entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZaiEndpointSpec {
    /// Stable endpoint identifier (`global`, `cn`, `coding-global`, or
    /// `coding-cn`).
    pub id: &'static str,
    /// OpenAI-compatible API base URL.
    pub base_url: &'static str,
    /// Candidate probe models, in the order they must be tried.
    pub models: &'static [&'static str],
    /// Human-facing endpoint label.
    pub label: &'static str,
}

impl ZaiEndpointSpec {
    /// Return this endpoint's ordered probe candidates.
    pub const fn probe_models(self) -> &'static [&'static str] {
        self.models
    }
}

/// A successful Z.AI endpoint probe.
///
/// The result retains the endpoint metadata and the first candidate model
/// accepted by that endpoint, matching `auth.py` lines 694–731.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZaiEndpointResult {
    pub id: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub label: &'static str,
}

impl ZaiEndpointResult {
    /// Stable endpoint identifier, provided as a descriptive alias for `id`.
    pub const fn endpoint_id(self) -> &'static str {
        self.id
    }
}

const GLOBAL_MODELS: &[&str] = &["glm-5"];
const CODING_MODELS: &[&str] = &["glm-5.2", "glm-5.1", "glm-5v-turbo", "glm-4.7"];

/// Z.AI endpoints in the upstream static priority order.
pub const ZAI_ENDPOINTS: &[ZaiEndpointSpec] = &[
    ZaiEndpointSpec {
        id: "global",
        base_url: "https://api.z.ai/api/paas/v4",
        models: GLOBAL_MODELS,
        label: "Global",
    },
    ZaiEndpointSpec {
        id: "cn",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        models: GLOBAL_MODELS,
        label: "China",
    },
    ZaiEndpointSpec {
        id: "coding-global",
        base_url: "https://api.z.ai/api/coding/paas/v4",
        models: CODING_MODELS,
        label: "Global (Coding Plan)",
    },
    ZaiEndpointSpec {
        id: "coding-cn",
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4",
        models: CODING_MODELS,
        label: "China (Coding Plan)",
    },
];

/// Return the static Z.AI endpoint table.
pub const fn zai_endpoint_specs() -> &'static [ZaiEndpointSpec] {
    ZAI_ENDPOINTS
}

/// Probe one endpoint using an injected transport-neutral request callback.
///
/// The callback is invoked once per candidate model, in model-list order.
/// Returning `true` means that the candidate produced an HTTP-200-equivalent
/// success.  Exceptions and transport details belong to the caller; a
/// callback that cannot complete a request should return `false`, preserving
/// upstream fail-open behavior from `auth.py` lines 705–731.
pub fn probe_zai_endpoint<F>(
    endpoint: &ZaiEndpointSpec,
    mut request_succeeds: F,
) -> Option<ZaiEndpointResult>
where
    F: FnMut(&ZaiEndpointSpec, &str) -> bool,
{
    endpoint
        .probe_models()
        .iter()
        .copied()
        .find(|model| request_succeeds(endpoint, model))
        .map(|model| ZaiEndpointResult {
            id: endpoint.id,
            base_url: endpoint.base_url,
            model,
            label: endpoint.label,
        })
}

/// Probe all Z.AI endpoints and choose the highest-priority success.
///
/// The injected callback may complete its underlying work in any order, but
/// this chooser evaluates endpoint results in [`ZAI_ENDPOINTS`] order and
/// therefore never lets completion order change selection.  Candidate models
/// within each endpoint are still tried in their declared order.  If every
/// request fails, returns `None`, matching `auth.py` lines 734–779.
pub fn choose_zai_endpoint<F>(mut request_succeeds: F) -> Option<ZaiEndpointResult>
where
    F: FnMut(&ZaiEndpointSpec, &str) -> bool,
{
    ZAI_ENDPOINTS
        .iter()
        .find_map(|endpoint| probe_zai_endpoint(endpoint, &mut request_succeeds))
}

/// Resolve a Z.AI base URL without performing I/O.
///
/// This is the pure precedence seam for `auth.py` lines 784–815:
/// a non-empty explicit environment override wins; without an API key the
/// profile default is returned; otherwise an injected cached URL wins over an
/// injected detected URL, with the default as the fail-open fallback.
pub fn resolve_zai_base_url(
    api_key: &str,
    default_url: &str,
    env_override: &str,
    cached_url: Option<&str>,
    detected_url: Option<&str>,
) -> String {
    if !env_override.trim().is_empty() {
        return env_override.to_owned();
    }
    if api_key.trim().is_empty() {
        return default_url.to_owned();
    }
    cached_url
        .filter(|url| !url.trim().is_empty())
        .or_else(|| detected_url.filter(|url| !url.trim().is_empty()))
        .unwrap_or(default_url)
        .to_owned()
}

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("zai");
    profile.aliases = vec!["glm".into(), "z-ai".into(), "z.ai".into(), "zhipu".into()];
    profile.display_name = "Z.AI (GLM)".into();
    profile.description = "Z.AI / GLM — Zhipu AI models".into();
    profile.signup_url = "https://z.ai/".into();
    profile.env_vars = vec![
        "GLM_API_KEY".into(),
        "ZAI_API_KEY".into(),
        "Z_AI_API_KEY".into(),
    ];
    profile.fallback_models = vec!["glm-5.2".into(), "glm-5".into(), "glm-4-9b".into()];
    profile.base_url = "https://api.z.ai/api/paas/v4".into();
    profile.default_aux_model = "glm-4.5-flash".into();
    // PARITY: ZaiProfile owns GLM version gating, thinking toggles, and
    // GLM-5.2's top-level reasoning_effort mapping.
    profile.zai_reasoning = true;
    profile
}
