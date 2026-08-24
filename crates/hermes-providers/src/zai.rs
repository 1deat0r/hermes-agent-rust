//! Z.AI / GLM provider profile.
//!
//! PARITY: plugins/model-providers/zai/__init__.py @ b9aa928.

use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crate::base::ProviderProfile;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

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
/// Probe one endpoint with the concrete blocking HTTP transport.
///
/// This mirrors `_probe_single_zai_endpoint` in `hermes_cli/auth.py`
/// lines 694–731: candidates are attempted in order, each request is a
/// `POST` to `/chat/completions`, and only HTTP 200 is considered success.
/// Request and transport errors fail open to the next candidate.
pub fn probe_zai_endpoint_http(
    endpoint: &ZaiEndpointSpec,
    api_key: &str,
    timeout: Duration,
) -> Option<ZaiEndpointResult> {
    probe_zai_endpoint_http_at(endpoint, endpoint.base_url, api_key, timeout)
}

/// Testable concrete HTTP probe with an explicit base URL.
///
/// The endpoint table intentionally stores static production URLs. This seam
/// keeps that public table unchanged while allowing deterministic local HTTP
/// tests without changing endpoint metadata ownership.
pub fn probe_zai_endpoint_http_at(
    endpoint: &ZaiEndpointSpec,
    base_url: &str,
    api_key: &str,
    timeout: Duration,
) -> Option<ZaiEndpointResult> {
    let client = Client::builder().timeout(timeout).build().ok()?;
    Some(probe_zai_endpoint(endpoint, |_, model| {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        client
            .post(url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {api_key}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({
                    "model": model,
                    "stream": false,
                    "max_tokens": 1,
                    "messages": [{"role": "user", "content": "ping"}],
                }))
                .unwrap_or_default(),
            )
            .send()
            .map(|response| response.status() == StatusCode::OK)
            .unwrap_or(false)
    })?)
}

/// Probe all endpoints concurrently using an injected request callback.
///
/// This is the dependency-free equivalent of `detect_zai_endpoint` in
/// `hermes_cli/auth.py` lines 734–779. Each endpoint gets one worker and
/// preserves its candidate-model order. Results are consumed in completion
/// order but selected only once every higher-priority endpoint has finished;
/// this preserves static endpoint priority while allowing early return
/// without joining slower lower-priority workers.
pub fn detect_zai_endpoint_with_probe<F>(request_succeeds: F) -> Option<ZaiEndpointResult>
where
    F: Fn(&ZaiEndpointSpec, &str) -> bool + Send + Sync + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let request_succeeds = Arc::new(request_succeeds);

    for (index, endpoint) in ZAI_ENDPOINTS.iter().enumerate() {
        let sender = sender.clone();
        let request_succeeds = Arc::clone(&request_succeeds);
        thread::spawn(move || {
            let result = probe_zai_endpoint(endpoint, |endpoint, model| {
                request_succeeds(endpoint, model)
            });
            let _ = sender.send((index, result));
        });
    }
    drop(sender);

    let mut finished = [false; ZAI_ENDPOINTS.len()];
    let mut results = [None; ZAI_ENDPOINTS.len()];
    while let Ok((index, result)) = receiver.recv() {
        finished[index] = true;
        results[index] = result;
        for index in 0..ZAI_ENDPOINTS.len() {
            if !finished[index] {
                break;
            }
            if let Some(result) = results[index] {
                return Some(result);
            }
        }
    }

    results.into_iter().flatten().next()
}

/// Detect the highest-priority endpoint with the concrete HTTP transport.
///
/// Client construction errors and every request failure are treated as
/// unavailable endpoints, matching the upstream fail-open detector.
pub fn detect_zai_endpoint(api_key: &str, timeout: Duration) -> Option<ZaiEndpointResult> {
    let client = Client::builder().timeout(timeout).build().ok()?;
    let api_key = api_key.to_owned();
    detect_zai_endpoint_with_probe(move |endpoint, model| {
        let url = format!(
            "{}/chat/completions",
            endpoint.base_url.trim_end_matches('/')
        );
        client
            .post(url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {api_key}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({
                    "model": model,
                    "stream": false,
                    "max_tokens": 1,
                    "messages": [{"role": "user", "content": "ping"}],
                }))
                .unwrap_or_default(),
            )
            .send()
            .map(|response| response.status() == StatusCode::OK)
            .unwrap_or(false)
    })
}

/// Return the first 16 lowercase hexadecimal characters of the API key's
/// SHA-256 digest, matching `hashlib.sha256(api_key.encode()).hexdigest()[:16]`.
pub fn zai_api_key_hash(api_key: &str) -> String {
    let mut digest = format!("{:x}", Sha256::digest(api_key.as_bytes()));
    digest.truncate(16);
    digest
}

/// Read a cached detected endpoint from a Z.AI provider-state JSON map.
///
/// The cache is valid only when `detected_endpoint` is an object containing a
/// non-empty string `base_url` and a `key_hash` matching the supplied API key.
/// Malformed or mismatched state is treated as an unavailable cache entry.
pub fn cached_zai_base_url(provider_state: &Map<String, Value>, api_key: &str) -> Option<String> {
    let endpoint = provider_state.get("detected_endpoint")?.as_object()?;
    let base_url = endpoint.get("base_url")?.as_str()?;
    let key_hash = endpoint.get("key_hash")?.as_str()?;
    if base_url.is_empty() || key_hash != zai_api_key_hash(api_key) {
        return None;
    }
    Some(base_url.to_owned())
}

/// Serialize a successful endpoint detection into the persisted cache fields.
///
/// The returned map intentionally contains exactly the fields written by
/// upstream `auth.py`: `base_url`, `endpoint_id`, `model`, `label`, and
/// `key_hash`.
pub fn serialize_zai_endpoint_result(
    result: &ZaiEndpointResult,
    api_key: &str,
) -> Map<String, Value> {
    Map::from_iter([
        (
            "base_url".to_owned(),
            Value::String(result.base_url.to_owned()),
        ),
        (
            "endpoint_id".to_owned(),
            Value::String(result.id.to_owned()),
        ),
        ("model".to_owned(), Value::String(result.model.to_owned())),
        ("label".to_owned(), Value::String(result.label.to_owned())),
        (
            "key_hash".to_owned(),
            Value::String(zai_api_key_hash(api_key)),
        ),
    ])
}

/// Resolve a Z.AI base URL using an optional provider-state cache.
///
/// A non-empty explicit override wins first. If no API key is configured, the
/// default is returned without consulting the cache. Otherwise a matching
/// cached endpoint wins, followed by a newly detected URL, and finally the
/// default fail-open fallback.
pub fn resolve_zai_base_url_with_cache(
    api_key: &str,
    default_url: &str,
    env_override: &str,
    provider_state: Option<&Map<String, Value>>,
    detected_url: Option<&str>,
) -> String {
    if !env_override.is_empty() {
        return env_override.to_owned();
    }
    if api_key.is_empty() {
        return default_url.to_owned();
    }
    let cached_url = provider_state.and_then(|state| cached_zai_base_url(state, api_key));
    resolve_zai_base_url(
        api_key,
        default_url,
        "",
        cached_url.as_deref(),
        detected_url,
    )
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
    if !env_override.is_empty() {
        return env_override.to_owned();
    }
    if api_key.is_empty() {
        return default_url.to_owned();
    }
    cached_url
        .filter(|url| !url.is_empty())
        .or_else(|| detected_url.filter(|url| !url.is_empty()))
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
