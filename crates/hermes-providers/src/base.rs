//! Provider profile base surface.
//!
//! PARITY: `providers/base.py` @ b9aa928. Profiles are declarative; provider
//! clients, credential rotation, and streaming remain outside this module.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, LOCATION, USER_AGENT,
};
use reqwest::{StatusCode, Url};
use serde_json::{Map, Value};

/// Rust representation of Python's `OMIT_TEMPERATURE` identity sentinel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FixedTemperature {
    CallerDefault,
    Omit,
    Value(f64),
}

/// Sentinel for providers that must omit the temperature field entirely.
pub const OMIT_TEMPERATURE: FixedTemperature = FixedTemperature::Omit;

/// Model-catalog request shape selected by a provider profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelsFetchMode {
    /// OpenAI-compatible `/models` discovery with optional Bearer auth.
    Standard,
    /// Native Anthropic `/v1/models` discovery with `x-api-key` auth.
    Anthropic,
}

/// Declarative provider profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderProfile {
    pub name: String,
    pub api_mode: String,
    pub aliases: Vec<String>,
    pub display_name: String,
    pub description: String,
    pub signup_url: String,
    pub env_vars: Vec<String>,
    pub base_url: String,
    pub models_url: String,
    pub auth_type: String,
    pub supports_health_check: bool,
    pub supports_vision: bool,
    pub supports_vision_tool_messages: bool,
    pub supports_prompt_cache_key: bool,
    pub fallback_models: Vec<String>,
    pub hostname: String,
    pub default_headers: BTreeMap<String, String>,
    pub fixed_temperature: FixedTemperature,
    pub default_max_tokens: Option<u32>,
    pub default_aux_model: String,
    /// Use Actual Computer's environment-aware `/models` catalog hook.
    pub actual_catalog: bool,
    /// Disable REST model discovery when a provider uses a separate SDK.
    pub models_fetch_disabled: bool,
    /// Select a provider-specific model-catalog request shape.
    pub models_fetch_mode: ModelsFetchMode,
    /// Translate Hermes reasoning into Gemini's native thinking config.
    pub gemini_thinking: bool,
    /// Translate Hermes reasoning into Vertex's nested Google config.
    pub vertex_thinking: bool,
    /// Resolve DeepInfra's default vision model from its tagged live catalog.
    pub deepinfra_vision: bool,
    /// Translate DeepSeek V4+ reasoning into its thinking/reasoning wire shape.
    pub deepseek_reasoning: bool,
    /// Add Nous Portal tags/sticky routing and apply its reasoning omission rule.
    pub nous_portal: bool,
    /// Translate Ollama Cloud reasoning into top-level reasoning_effort.
    pub ollama_cloud_reasoning: bool,
    /// Translate MiniMax-M3 reasoning for the global OpenAI-compatible route.
    pub minimax_reasoning: bool,
    /// Route Copilot reasoning through the live model-catalog effort list.
    pub copilot_reasoning: bool,
    /// Route the reasoning configuration through `extra_body.reasoning`.
    pub reasoning_passthrough: bool,
}

impl ProviderProfile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            api_mode: "chat_completions".into(),
            aliases: Vec::new(),
            display_name: String::new(),
            description: String::new(),
            signup_url: String::new(),
            env_vars: Vec::new(),
            base_url: String::new(),
            models_url: String::new(),
            auth_type: "api_key".into(),
            supports_health_check: true,
            supports_vision: false,
            supports_vision_tool_messages: true,
            supports_prompt_cache_key: false,
            fallback_models: Vec::new(),
            hostname: String::new(),
            default_headers: BTreeMap::new(),
            fixed_temperature: FixedTemperature::CallerDefault,
            default_max_tokens: None,
            default_aux_model: String::new(),
            actual_catalog: false,
            models_fetch_disabled: false,
            models_fetch_mode: ModelsFetchMode::Standard,
            gemini_thinking: false,
            vertex_thinking: false,
            deepinfra_vision: false,
            deepseek_reasoning: false,
            nous_portal: false,
            ollama_cloud_reasoning: false,
            minimax_reasoning: false,
            copilot_reasoning: false,
            reasoning_passthrough: false,
        }
    }

    pub fn get_hostname(&self) -> String {
        if !self.hostname.is_empty() {
            return self.hostname.clone();
        }
        if self.base_url.is_empty() {
            return String::new();
        }

        Url::parse(&self.base_url)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .unwrap_or_default()
    }

    pub fn prepare_messages(&self, messages: &[Value]) -> Vec<Value> {
        messages.to_vec()
    }

    pub fn build_extra_body(
        &self,
        session_id: Option<&str>,
        context: &Map<String, Value>,
    ) -> Map<String, Value> {
        if self.nous_portal {
            return build_nous_extra_body(session_id, context);
        }
        if self.vertex_thinking {
            return build_vertex_extra_body(context);
        }
        if self.gemini_thinking {
            return build_gemini_extra_body(self, context);
        }
        Map::new()
    }

    pub fn build_api_kwargs_extras(
        &self,
        reasoning_config: Option<&Map<String, Value>>,
        context: &Map<String, Value>,
    ) -> (Map<String, Value>, Map<String, Value>) {
        if self.nous_portal {
            return build_nous_api_kwargs_extras(reasoning_config, context);
        }
        if self.deepseek_reasoning {
            return build_deepseek_reasoning(reasoning_config, context);
        }
        if self.ollama_cloud_reasoning {
            return build_ollama_cloud_reasoning(reasoning_config, context);
        }
        if self.minimax_reasoning {
            return build_minimax_reasoning(reasoning_config, context);
        }
        if self.copilot_reasoning {
            return build_copilot_reasoning(reasoning_config, context);
        }
        if !self.reasoning_passthrough {
            return (Map::new(), Map::new());
        }

        // PARITY: VercelAIGatewayProfile defaults its typed
        // `supports_reasoning=True` parameter when the transport does not
        // supply the context key, and emits no reasoning body when it is false.
        let supports_reasoning = context
            .get("supports_reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !supports_reasoning {
            return (Map::new(), Map::new());
        }

        let reasoning = reasoning_config.cloned().unwrap_or_else(|| {
            Map::from_iter([
                ("enabled".into(), Value::Bool(true)),
                ("effort".into(), Value::String("medium".into())),
            ])
        });
        let mut extra_body = Map::new();
        extra_body.insert("reasoning".into(), Value::Object(reasoning));
        return (extra_body, Map::new());
    }

    pub fn default_vision_model(&self) -> Option<String> {
        if self.deepinfra_vision {
            return deepinfra_default_vision_model();
        }
        None
    }

    pub fn get_max_tokens(&self, _model: Option<&str>) -> Option<u32> {
        self.default_max_tokens
    }

    pub fn fetch_models(
        &self,
        api_key: Option<&str>,
        base_url: Option<&str>,
        timeout: f64,
    ) -> Option<Vec<String>> {
        if self.actual_catalog {
            return fetch_actual_models(api_key, base_url, timeout, &self.base_url);
        }

        // PARITY: BedrockProfile overrides the upstream method and always
        // returns None because model discovery uses the AWS SDK, not REST.
        if self.models_fetch_disabled {
            return None;
        }

        if self.models_fetch_mode == ModelsFetchMode::Anthropic {
            // PARITY: AnthropicProfile.fetch_models() requires a non-empty
            // key, ignores caller base_url, and probes its fixed native URL.
            if api_key.map_or(true, str::is_empty) {
                return None;
            }
            let endpoint = if self.models_url.trim().is_empty() {
                "https://api.anthropic.com/v1/models"
            } else {
                // Test/integration seam: a profile clone may supply an
                // explicit endpoint without changing the production default.
                self.models_url.trim()
            };
            return match self.fetch_models_inner(
                api_key,
                endpoint,
                timeout,
                ModelsFetchMode::Anthropic,
            ) {
                Ok(models) => Some(models),
                Err(error) => {
                    // PARITY: the native Anthropic probe is fail-open and
                    // records failures only at debug level.
                    log::debug!("fetch_models({}): {}", self.name, error);
                    None
                }
            };
        }

        // PARITY: Python's `base_url or self.base_url` treats an empty caller
        // override as absent, and `models_url` wins over either base URL.
        let effective_base = base_url
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.base_url);
        let explicit_models_url = self.models_url.trim();
        let endpoint = if explicit_models_url.is_empty() {
            if effective_base.is_empty() {
                return None;
            }
            format!("{}/models", effective_base.trim_end_matches('/'))
        } else {
            explicit_models_url.to_owned()
        };

        match self.fetch_models_inner(api_key, &endpoint, timeout, ModelsFetchMode::Standard) {
            Ok(models) => Some(models),
            Err(error) => {
                // PARITY: the upstream catalog probe is deliberately
                // fail-open and records failures only at debug level.
                log::debug!("fetch_models({}): {}", self.name, error);
                None
            }
        }
    }

    fn fetch_models_inner(
        &self,
        api_key: Option<&str>,
        endpoint: &str,
        timeout: f64,
        mode: ModelsFetchMode,
    ) -> Result<Vec<String>, String> {
        let timeout = if timeout.is_finite() && timeout >= 0.0 {
            Duration::from_secs_f64(timeout)
        } else {
            return Err("invalid timeout".into());
        };
        let mut current_url = Url::parse(endpoint).map_err(|error| error.to_string())?;
        let original_origin = url_origin(&current_url);

        let mut headers = HeaderMap::new();
        match mode {
            ModelsFetchMode::Standard => {
                if let Some(api_key) = api_key {
                    let value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                        .map_err(|error| error.to_string())?;
                    headers.insert(AUTHORIZATION, value);
                }
            }
            ModelsFetchMode::Anthropic => {
                let api_key = api_key.filter(|value| !value.is_empty()).ok_or_else(|| {
                    "Anthropic model discovery requires a non-empty API key".to_owned()
                })?;
                let value = HeaderValue::from_str(api_key).map_err(|error| error.to_string())?;
                headers.insert(HeaderName::from_static("x-api-key"), value);
                headers.insert(
                    HeaderName::from_static("anthropic-version"),
                    HeaderValue::from_static("2023-06-01"),
                );
            }
        }
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        if mode == ModelsFetchMode::Standard {
            let user_agent = profile_user_agent();
            headers.insert(
                USER_AGENT,
                HeaderValue::from_str(&user_agent).map_err(|error| error.to_string())?,
            );
            for (name, value) in &self.default_headers {
                let name =
                    HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?;
                let value = HeaderValue::from_str(value).map_err(|error| error.to_string())?;
                headers.insert(name, value);
            }
        }

        // urllib's secure opener preserves an installed application's proxy,
        // TLS, cookie, protocol-handler, and instrumentation policy while
        // replacing redirect handling. The Rust CLI owner is not present yet;
        // reqwest supplies the transport and environment proxy/TLS behavior,
        // with automatic redirects disabled so the header allowlist can be
        // applied before each redirected request.
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|error| error.to_string())?;

        // HTTPRedirectHandler defaults to ten redirects. Keep the same bound
        // and compare every hop with the *original* origin, matching
        // SafeCredentialRedirectHandler rather than only comparing adjacent
        // URLs.
        for _ in 0..=10 {
            let response = client
                .get(current_url.clone())
                .headers(headers.clone())
                .send()
                .map_err(|error| error.to_string())?;

            // PARITY: urllib's HTTPRedirectHandler follows these five
            // redirect statuses; other 3xx responses are treated as final.
            if matches!(
                response.status(),
                StatusCode::MOVED_PERMANENTLY
                    | StatusCode::FOUND
                    | StatusCode::SEE_OTHER
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::PERMANENT_REDIRECT
            ) {
                let location = response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| "redirect missing Location".to_owned())?
                    .to_str()
                    .map_err(|error| error.to_string())?;
                let next_url = current_url
                    .join(location)
                    .map_err(|error| error.to_string())?;
                if url_origin(&next_url) != original_origin {
                    headers = cross_origin_safe_headers(&headers);
                }
                current_url = next_url;
                continue;
            }

            let response = response
                .error_for_status()
                .map_err(|error| error.to_string())?;
            let body = response.bytes().map_err(|error| error.to_string())?;
            let body = String::from_utf8(body.to_vec()).map_err(|error| error.to_string())?;
            let data: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
            let items = match data {
                Value::Array(items) if mode == ModelsFetchMode::Standard => items,
                Value::Object(mut object) => object
                    .remove("data")
                    .and_then(|value| value.as_array().cloned())
                    .unwrap_or_default(),
                _ => return Err("model catalog response is not an object or array".into()),
            };
            return Ok(items
                .into_iter()
                .filter_map(|item| match item {
                    Value::Object(object) => object
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    _ => None,
                })
                .collect());
        }

        Err("too many redirects".into())
    }
}

fn fetch_actual_models(
    api_key: Option<&str>,
    base_url: Option<&str>,
    timeout: f64,
    profile_base_url: &str,
) -> Option<Vec<String>> {
    // PARITY: ActualProfile prefers a non-empty ACTUAL_BASE_URL environment
    // override, then the caller base URL, then its hosted profile default.
    let environment_base_url = std::env::var("ACTUAL_BASE_URL").unwrap_or_default();
    let raw_base_url = if !environment_base_url.trim().is_empty() {
        environment_base_url.trim().to_owned()
    } else {
        base_url
            .filter(|value| !value.is_empty())
            .unwrap_or(profile_base_url)
            .to_owned()
    };
    let normalized_base_url = normalize_actual_base_url(&raw_base_url);
    if normalized_base_url.is_empty() {
        return None;
    }

    let timeout = if timeout.is_finite() && timeout >= 0.0 {
        Duration::from_secs_f64(timeout)
    } else {
        return None;
    };
    let endpoint = format!("{}/models", normalized_base_url.trim_end_matches('/'));
    let client = Client::builder().timeout(timeout).build().ok()?;
    let mut request = client
        .get(endpoint)
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, profile_user_agent());
    if let Some(api_key) = api_key.filter(|value| !value.is_empty()) {
        request = request.header(AUTHORIZATION, format!("Bearer {api_key}"));
    }

    let result = (|| {
        let response = request.send().map_err(|error| error.to_string())?;
        let response = response
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let payload = response.bytes().map_err(|error| error.to_string())?;
        let data: Value = serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
        let items = match data {
            Value::Array(items) => items,
            Value::Object(mut object) => match object.remove("data") {
                None => Vec::new(),
                Some(Value::Array(items)) => items,
                Some(Value::String(_) | Value::Object(_)) => Vec::new(),
                Some(_) => return Err("Actual model catalog data is not iterable".into()),
            },
            Value::String(_) => Vec::new(),
            _ => return Err("Actual model catalog response is not iterable".into()),
        };
        Ok::<Vec<String>, String>(
            items
                .into_iter()
                .filter_map(|item| match item {
                    Value::Object(object) => object
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    _ => None,
                })
                .collect(),
        )
    })();

    match result {
        Ok(models) => Some(models),
        Err(error) => {
            log::debug!("fetch_models(actual): {error}");
            None
        }
    }
}

fn normalize_actual_base_url(base_url: &str) -> String {
    // PARITY: `_normalize_actual_base_url` adds `/v1` only for Actual's
    // hosted root and recognized local roots; all other paths are preserved.
    let url = base_url.trim().trim_end_matches('/').to_owned();
    if url.is_empty() {
        return "https://api.actual.inc/v1".into();
    }

    let Ok(parsed) = Url::parse(&url) else {
        return url;
    };
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .trim_end_matches('.')
        .to_owned();
    let path = parsed.path().trim_end_matches('/');
    let is_root = path.is_empty() || path == "/";
    if (host == "api.actual.inc"
        || matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0"))
        && is_root
    {
        return format!("{url}/v1");
    }
    url
}

// PARITY: _DeepInfraProfile.default_vision_model() gates on
// DEEPINFRA_API_KEY, then delegates to the chat-tagged DeepInfra catalog.
// The catalog cache is process-global and keyed only by the effective base
// URL, matching hermes_cli.models; failures are cached briefly to avoid
// repeated blocking probes while offline.
fn deepinfra_default_vision_model() -> Option<String> {
    let api_key = std::env::var("DEEPINFRA_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let base_url = std::env::var("DEEPINFRA_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "https://api.deepinfra.com/v1/openai".into());
    let base_url = base_url.trim_end_matches('/').to_owned();
    let catalog = fetch_deepinfra_catalog(&base_url, &api_key)?;

    for item in catalog {
        let Some(model_id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(raw_metadata) = item.get("metadata") else {
            continue;
        };
        let Some(metadata) = raw_metadata.as_object() else {
            continue;
        };
        let Some(tags) = metadata.get("tags").and_then(Value::as_array) else {
            continue;
        };
        let tags: Vec<&str> = tags.iter().filter_map(Value::as_str).collect();
        let has_surface_tag = tags.iter().any(|tag| {
            matches!(
                *tag,
                "chat" | "embed" | "image-gen" | "tts" | "stt" | "video-gen"
            )
        });
        if has_surface_tag {
            if !tags.contains(&"chat") {
                continue;
            }
        } else if deepinfra_chat_id_is_excluded(model_id) {
            continue;
        }
        if tags.contains(&"vision") && !model_id.is_empty() {
            return Some(model_id.to_owned());
        }
    }
    None
}

type DeepInfraCatalogCache = HashMap<String, Vec<Value>>;
type DeepInfraNegativeCache = HashMap<String, Instant>;

static DEEPINFRA_CATALOG_CACHE: OnceLock<Mutex<DeepInfraCatalogCache>> = OnceLock::new();
static DEEPINFRA_NEGATIVE_CACHE: OnceLock<Mutex<DeepInfraNegativeCache>> = OnceLock::new();

fn fetch_deepinfra_catalog(base_url: &str, api_key: &str) -> Option<Vec<Value>> {
    let cache = DEEPINFRA_CATALOG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(catalog) = cache.lock().ok()?.get(base_url).cloned() {
        return Some(catalog);
    }

    let negative_cache = DEEPINFRA_NEGATIVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(last_failure) = negative_cache.lock().ok()?.get(base_url).copied() {
        if last_failure.elapsed() < Duration::from_secs(60) {
            return None;
        }
    }

    let endpoint = format!("{base_url}/models?filter=true&sort_by=hermes");
    let result = (|| {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|error| error.to_string())?;
        let mut request = client
            .get(endpoint)
            .header(USER_AGENT, profile_user_agent());
        if !api_key.is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {api_key}"));
        }
        let response = request
            .send()
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let payload: Value =
            serde_json::from_slice(&response.bytes().map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
        payload
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| "DeepInfra catalog response has no data list".to_owned())
    })();

    match result {
        Ok(catalog) => {
            if let Ok(mut cached) = cache.lock() {
                cached.insert(base_url.to_owned(), catalog.clone());
            }
            if let Ok(mut failures) = negative_cache.lock() {
                failures.remove(base_url);
            }
            Some(catalog)
        }
        Err(error) => {
            log::debug!("DeepInfra catalog fetch failed: {error}");
            if let Ok(mut failures) = negative_cache.lock() {
                failures.insert(base_url.to_owned(), Instant::now());
            }
            None
        }
    }
}

fn deepinfra_chat_id_is_excluded(model_id: &str) -> bool {
    let model_id = model_id.to_ascii_lowercase();
    [
        "embed",
        "rerank",
        "whisper",
        "stable-diffusion",
        "flux",
        "sdxl",
        "tts",
        "bark",
        "speech",
        "image-gen",
        "clip",
        "vit-",
        "dpt-",
    ]
    .iter()
    .any(|marker| model_id.contains(marker))
}

/// Return the user-agent used by model catalog probes.
pub fn profile_user_agent() -> String {
    // PARITY: `_profile_user_agent` lazily imports `hermes_cli.__version__`
    // and falls back to this stable string when that higher layer is absent.
    "hermes-cli".into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Origin {
    scheme: String,
    hostname: String,
    port: Option<u16>,
}

fn url_origin(url: &Url) -> Origin {
    Origin {
        scheme: url.scheme().to_ascii_lowercase(),
        hostname: url
            .host_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_owned(),
        port: url.port_or_known_default(),
    }
}

fn cross_origin_safe_headers(headers: &HeaderMap) -> HeaderMap {
    // PARITY: urllib_security.py's allowlist is intentionally narrow because
    // provider credentials can use arbitrary custom header names.
    let mut safe = HeaderMap::new();
    if let Some(value) = headers.get(ACCEPT) {
        safe.insert(ACCEPT, value.clone());
    }
    if let Some(value) = headers.get(USER_AGENT) {
        safe.insert(USER_AGENT, value.clone());
    }
    safe
}

fn build_gemini_extra_body(
    profile: &ProviderProfile,
    context: &Map<String, Value>,
) -> Map<String, Value> {
    // PARITY: GeminiProfile.build_extra_body() delegates to the transport's
    // `_build_gemini_thinking_config` helper and returns no body when the
    // resolved model is not a Gemini thinking model.
    let model = context
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let reasoning_config = context.get("reasoning_config").and_then(Value::as_object);
    let raw_thinking_config = build_gemini_thinking_config(model, reasoning_config);
    let Some(raw_thinking_config) = raw_thinking_config else {
        return Map::new();
    };

    let base_url = context
        .get("base_url")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(&profile.base_url);
    if profile.name == "gemini" && is_gemini_openai_compat_base_url(base_url) {
        let Some(thinking_config) = snake_case_gemini_thinking_config(&raw_thinking_config) else {
            return Map::new();
        };
        return nested_gemini_extra_body(thinking_config);
    }

    Map::from_iter([("thinking_config".into(), Value::Object(raw_thinking_config))])
}

fn build_vertex_extra_body(context: &Map<String, Value>) -> Map<String, Value> {
    // PARITY: VertexProfile uses the same Gemini helper but always routes the
    // result through `_snake_case_gemini_thinking_config` for its OpenAI API.
    let model = context
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let reasoning_config = context.get("reasoning_config").and_then(Value::as_object);
    let Some(raw_thinking_config) = build_gemini_thinking_config(model, reasoning_config) else {
        return Map::new();
    };
    let Some(thinking_config) = snake_case_gemini_thinking_config(&raw_thinking_config) else {
        return Map::new();
    };
    nested_gemini_extra_body(thinking_config)
}

fn nested_gemini_extra_body(thinking_config: Map<String, Value>) -> Map<String, Value> {
    let mut google = Map::new();
    google.insert("thinking_config".into(), Value::Object(thinking_config));
    let mut compatibility = Map::new();
    compatibility.insert("google".into(), Value::Object(google));
    let mut body = Map::new();
    body.insert("extra_body".into(), Value::Object(compatibility));
    body
}

fn build_gemini_thinking_config(
    model: &str,
    reasoning_config: Option<&Map<String, Value>>,
) -> Option<Map<String, Value>> {
    let reasoning_config = reasoning_config?;
    let mut normalized_model = model.trim().to_ascii_lowercase();
    if let Some(stripped) = normalized_model.strip_prefix("google/") {
        normalized_model = stripped.to_owned();
    }
    if !normalized_model.starts_with("gemini") {
        return None;
    }

    if reasoning_config
        .get("enabled")
        .and_then(Value::as_bool)
        .is_some_and(|enabled| !enabled)
    {
        return Some(Map::from_iter([(
            "includeThoughts".into(),
            Value::Bool(false),
        )]));
    }

    let effort = reasoning_config
        .get("effort")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .trim()
        .to_ascii_lowercase();
    if effort == "none" {
        return Some(Map::from_iter([(
            "includeThoughts".into(),
            Value::Bool(false),
        )]));
    }

    let mut thinking_config = Map::from_iter([("includeThoughts".into(), Value::Bool(true))]);
    if normalized_model.starts_with("gemini-2.5-") {
        return Some(thinking_config);
    }

    let effort = match effort.as_str() {
        "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra" => effort,
        _ => "medium".into(),
    };
    if normalized_model.starts_with("gemini-3") {
        if normalized_model.contains("flash") {
            let level = if matches!(effort.as_str(), "minimal" | "low") {
                "low"
            } else if matches!(effort.as_str(), "high" | "xhigh" | "max" | "ultra") {
                "high"
            } else {
                "medium"
            };
            thinking_config.insert("thinkingLevel".into(), Value::String(level.into()));
        } else if normalized_model.contains("pro") {
            let level = if matches!(effort.as_str(), "high" | "xhigh" | "max" | "ultra") {
                "high"
            } else {
                "low"
            };
            thinking_config.insert("thinkingLevel".into(), Value::String(level.into()));
        }
    }
    Some(thinking_config)
}

fn snake_case_gemini_thinking_config(config: &Map<String, Value>) -> Option<Map<String, Value>> {
    if config.is_empty() {
        return None;
    }
    let mut translated = Map::new();
    if let Some(Value::Bool(include_thoughts)) = config.get("includeThoughts") {
        translated.insert("include_thoughts".into(), Value::Bool(*include_thoughts));
    }
    if let Some(Value::String(thinking_level)) = config.get("thinkingLevel") {
        let thinking_level = thinking_level.trim();
        if !thinking_level.is_empty() {
            translated.insert(
                "thinking_level".into(),
                Value::String(thinking_level.to_ascii_lowercase()),
            );
        }
    }
    if let Some(value) = config.get("thinkingBudget") {
        let budget = value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            .or_else(|| value.as_f64().map(|value| value as i64));
        if let Some(budget) = budget {
            translated.insert("thinking_budget".into(), Value::Number(budget.into()));
        }
    }
    (!translated.is_empty()).then_some(translated)
}

fn is_gemini_openai_compat_base_url(base_url: &str) -> bool {
    let normalized = base_url.trim().trim_end_matches('/').to_ascii_lowercase();
    !normalized.is_empty()
        && normalized.contains("generativelanguage.googleapis.com")
        && normalized.ends_with("/openai")
}

fn build_deepseek_reasoning(
    reasoning_config: Option<&Map<String, Value>>,
    context: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    // PARITY: DeepSeekProfile applies this hook only to DeepSeek V4+ model
    // names, leaving V3 and unknown models' wire format untouched.
    let model = context
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !model.starts_with("deepseek-v") || model.starts_with("deepseek-v3") {
        return (Map::new(), Map::new());
    }

    let enabled = !reasoning_config
        .and_then(|config| config.get("enabled"))
        .and_then(Value::as_bool)
        .is_some_and(|enabled| !enabled);
    let mut thinking = Map::new();
    thinking.insert(
        "type".into(),
        Value::String(if enabled { "enabled" } else { "disabled" }.into()),
    );
    let mut extra_body = Map::new();
    extra_body.insert("thinking".into(), Value::Object(thinking));
    if !enabled {
        return (extra_body, Map::new());
    }

    let Some(effort) = reasoning_config
        .and_then(|config| config.get("effort"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|effort| !effort.is_empty())
        .map(str::to_ascii_lowercase)
    else {
        return (extra_body, Map::new());
    };
    let effort = match effort.as_str() {
        "xhigh" | "max" | "ultra" => "max",
        "low" | "medium" | "high" => effort.as_str(),
        _ => return (extra_body, Map::new()),
    };
    let mut top_level = Map::new();
    top_level.insert("reasoning_effort".into(), Value::String(effort.into()));
    (extra_body, top_level)
}

fn build_ollama_cloud_reasoning(
    reasoning_config: Option<&Map<String, Value>>,
    context: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    // PARITY: OllamaCloudProfile gates the top-level reasoning_effort field
    // on the transport's native /api/show thinking capability.
    let supports_reasoning = context
        .get("supports_reasoning")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !supports_reasoning {
        return (Map::new(), Map::new());
    }

    let Some(reasoning_config) = reasoning_config else {
        return (Map::new(), Map::new());
    };
    let enabled = !reasoning_config
        .get("enabled")
        .and_then(Value::as_bool)
        .is_some_and(|enabled| !enabled);
    if !enabled {
        return (
            Map::new(),
            Map::from_iter([("reasoning_effort".into(), Value::String("none".into()))]),
        );
    }

    let Some(effort) = reasoning_config
        .get("effort")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|effort| !effort.is_empty())
        .map(str::to_ascii_lowercase)
    else {
        return (Map::new(), Map::new());
    };
    let effort = match effort.as_str() {
        "none" => "none",
        "xhigh" | "max" | "ultra" => "max",
        "low" | "medium" | "high" => effort.as_str(),
        _ => return (Map::new(), Map::new()),
    };
    (
        Map::new(),
        Map::from_iter([("reasoning_effort".into(), Value::String(effort.into()))]),
    )
}

fn build_minimax_reasoning(
    reasoning_config: Option<&Map<String, Value>>,
    context: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    // PARITY: MiniMaxProfile accepts only MiniMax-M3 (or its provider slug)
    // on the exact global `api.minimax.io/v1` OpenAI-compatible route.
    let model = context
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let base_url = context
        .get("base_url")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !is_minimax_global_openai_base_url(base_url)
        || !matches!(model.as_str(), "minimax-m3" | "minimax/minimax-m3")
    {
        return (Map::new(), Map::new());
    }

    let mut extra_body = Map::from_iter([("reasoning_split".into(), Value::Bool(true))]);
    if let Some(reasoning_config) = reasoning_config {
        let disabled = reasoning_config
            .get("enabled")
            .and_then(Value::as_bool)
            .is_some_and(|enabled| !enabled);
        let thinking_type = if disabled { "disabled" } else { "adaptive" };
        extra_body.insert(
            "thinking".into(),
            Value::Object(Map::from_iter([(
                "type".into(),
                Value::String(thinking_type.into()),
            )])),
        );
    }
    (extra_body, Map::new())
}

fn is_minimax_global_openai_base_url(base_url: &str) -> bool {
    let Ok(parsed) = Url::parse(base_url.trim()) else {
        return false;
    };
    let Some(hostname) = parsed.host_str() else {
        return false;
    };
    if !hostname.eq_ignore_ascii_case("api.minimax.io") {
        return false;
    }
    parsed
        .path()
        .trim_end_matches('/')
        .eq_ignore_ascii_case("/v1")
}

fn build_nous_extra_body(
    session_id: Option<&str>,
    context: &Map<String, Value>,
) -> Map<String, Value> {
    // PARITY: NousProfile delegates product/client/conversation tags to
    // `agent.portal_tags.nous_portal_tags()`. The CLI version provider is a
    // future higher-layer seam; b9aa928's pinned hermes_cli version is used
    // until that seam is available.
    let mut tags = vec![
        Value::String("product=hermes-agent".into()),
        Value::String("client=hermes-client-v0.20.0".into()),
    ];
    let effective_session = context
        .get("conversation_context")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| session_id.filter(|value| !value.is_empty()));
    if let Some(session_id) = effective_session {
        tags.push(Value::String(format!("conversation={session_id}")));
    }

    let mut body = Map::new();
    body.insert("tags".into(), Value::Array(tags));

    let sticky_key = nous_cache_scope_from_session_id(effective_session);
    if !sticky_key.is_empty() {
        body.insert("session_id".into(), Value::String(sticky_key));
    }

    if let Some(provider_preferences) = context.get("provider_preferences") {
        if json_truthy(provider_preferences) {
            body.insert("provider".into(), provider_preferences.clone());
        }
    }
    body
}

fn build_nous_api_kwargs_extras(
    reasoning_config: Option<&Map<String, Value>>,
    context: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    // PARITY: NousProfile emits the standard reasoning body only when the
    // transport says the model supports reasoning, and intentionally omits
    // it for an explicit `enabled=False` configuration.
    let supports_reasoning = context
        .get("supports_reasoning")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !supports_reasoning {
        return (Map::new(), Map::new());
    }

    let mut extra_body = Map::new();
    if let Some(reasoning_config) = reasoning_config {
        if reasoning_config
            .get("enabled")
            .and_then(Value::as_bool)
            .is_some_and(|enabled| !enabled)
        {
            return (extra_body, Map::new());
        }
        extra_body.insert("reasoning".into(), Value::Object(reasoning_config.clone()));
    } else {
        extra_body.insert(
            "reasoning".into(),
            serde_json::json!({"enabled": true, "effort": "medium"}),
        );
    }
    (extra_body, Map::new())
}

fn nous_cache_scope_from_session_id(session_id: Option<&str>) -> String {
    // PARITY: `_cache_scope_from_session_id` strips only the timestamp from
    // cron_<job>_<YYYYMMDD>_<HHMMSS>; all other session IDs are unchanged.
    let session_id = session_id.unwrap_or_default();
    let Some((before_time, time)) = session_id.rsplit_once('_') else {
        return session_id.into();
    };
    let Some((prefix, date)) = before_time.rsplit_once('_') else {
        return session_id.into();
    };
    if prefix.starts_with("cron_")
        && prefix.len() > "cron_".len()
        && date.len() == 8
        && time.len() == 6
        && date.bytes().all(|byte| byte.is_ascii_digit())
        && time.bytes().all(|byte| byte.is_ascii_digit())
    {
        prefix.into()
    } else {
        session_id.into()
    }
}

fn json_truthy(value: &Value) -> bool {
    // Python's `if provider_preferences` treats empty containers and zero as
    // false; preserve that behavior for the JSON context adapter.
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn build_copilot_reasoning(
    reasoning_config: Option<&Map<String, Value>>,
    context: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    // PARITY: CopilotProfile delegates to
    // `hermes_cli.models.github_model_reasoning_efforts(model)`. The Rust
    // CLI/model crate is not present yet, so the supported effort list is an
    // explicit injected context seam for the same fail-open decision.
    let supports_reasoning = context
        .get("supports_reasoning")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let model = context
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !supports_reasoning || model.is_empty() {
        return (Map::new(), Map::new());
    }

    let supported_efforts: Vec<&str> = context
        .get("supported_efforts")
        .and_then(Value::as_array)
        .map(|efforts| efforts.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if supported_efforts.is_empty() {
        return (Map::new(), Map::new());
    }

    let has_reasoning_config = reasoning_config.is_some_and(|config| !config.is_empty());
    if !has_reasoning_config {
        let mut reasoning = Map::new();
        reasoning.insert("effort".into(), Value::String("medium".into()));
        let mut extra_body = Map::new();
        extra_body.insert("reasoning".into(), Value::Object(reasoning));
        return (extra_body, Map::new());
    }

    let mut effort = reasoning_config
        .and_then(|config| config.get("effort"))
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_owned();
    if !supported_efforts.contains(&effort.as_str()) {
        if effort == "xhigh" && supported_efforts.contains(&"high") {
            effort = "high".into();
        } else if effort == "minimal" && supported_efforts.contains(&"low") {
            effort = "low".into();
        } else if supported_efforts.contains(&"medium") {
            effort = "medium".into();
        } else {
            effort = supported_efforts[0].into();
        }
    }
    if !supported_efforts.contains(&effort.as_str()) {
        return (Map::new(), Map::new());
    }

    let mut reasoning = Map::new();
    reasoning.insert("effort".into(), Value::String(effort));
    let mut extra_body = Map::new();
    extra_body.insert("reasoning".into(), Value::Object(reasoning));
    (extra_body, Map::new())
}
