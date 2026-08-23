//! Provider profile base surface.
//!
//! PARITY: `providers/base.py` @ b9aa928. Profiles are declarative; provider
//! clients, credential rotation, and streaming remain outside this module.

use std::collections::BTreeMap;
use std::time::Duration;

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
    /// Disable REST model discovery when a provider uses a separate SDK.
    pub models_fetch_disabled: bool,
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
            models_fetch_disabled: false,
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
        _session_id: Option<&str>,
        _context: &Map<String, Value>,
    ) -> Map<String, Value> {
        Map::new()
    }

    pub fn build_api_kwargs_extras(
        &self,
        _reasoning_config: Option<&Map<String, Value>>,
        _context: &Map<String, Value>,
    ) -> (Map<String, Value>, Map<String, Value>) {
        (Map::new(), Map::new())
    }

    pub fn default_vision_model(&self) -> Option<String> {
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
        // PARITY: BedrockProfile overrides the upstream method and always
        // returns None because model discovery uses the AWS SDK, not REST.
        if self.models_fetch_disabled {
            return None;
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

        match self.fetch_models_inner(api_key, &endpoint, timeout) {
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
    ) -> Result<Vec<String>, String> {
        let timeout = if timeout.is_finite() && timeout >= 0.0 {
            Duration::from_secs_f64(timeout)
        } else {
            return Err("invalid timeout".into());
        };
        let mut current_url = Url::parse(endpoint).map_err(|error| error.to_string())?;
        let original_origin = url_origin(&current_url);

        let mut headers = HeaderMap::new();
        if let Some(api_key) = api_key {
            let value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| error.to_string())?;
            headers.insert(AUTHORIZATION, value);
        }
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
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
                Value::Array(items) => items,
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
