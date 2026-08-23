//! Shared FAL.ai SDK plumbing.
//!
//! PARITY: tools/fal_common.py @ b9aa928 (163 LOC, ported 1:1).
//!
//! Holds the stateless atoms every FAL-backed tool needs: the lazy
//! `fal_client` import hook, the managed-queue URL normalizer, the HTTP
//! status extractor, and the managed sync-client wrapper.
//!
//! SEAMS (documented): the Python `fal_client` third-party package is not
//! ported. The Rust equivalents are:
//!  * [`set_fal_client_provider`] — the "import machinery". A provider that
//!    returns a [`FalClientModule`] stands in for an installed `fal_client`
//!    package; `None` means the package is genuinely unavailable (the
//!    upstream `import fal_client` ImportError path).
//!  * [`set_ensure_hook`] — mirrors `tools.lazy_deps.ensure("image.fal")`.
//!    lazy_deps isn't ported; with no hook installed the ensure step is
//!    skipped, exactly like an absent `tools.lazy_deps` module.
//!  * The `FalClientModule` primitives are function slots driven by the
//!    future FAL-tail port, mirroring how upstream reads attributes off the
//!    `fal_client` module (a missing attribute is a None probe with the same
//!    RuntimeError text).
//!
//! The stateful pieces (cache globals, `_managed_fal_client*` selectors,
//! `_submit_fal_request`) intentionally stay on the image-generation tool
//! (upstream `tools.image_generation_tool`), exactly as documented upstream.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

/// Opaque FAL HTTP transport handle (mirrors `fal_client.SyncClient._client`).
/// Implemented by the FAL-tail port (or a test double).
pub trait HttpClientLike: Send + Sync {}

/// Error values from the managed-queue helpers. Upstream raises
/// `ValueError` / `RuntimeError`; the message text is preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedFalError {
    /// `ValueError("Managed FAL queue origin is required")`.
    QueueOriginRequired,
    /// `RuntimeError(msg)` from a missing-attribute probe or a transported
    /// request/raise-for-status failure (the inner text is the exception
    /// message upstream would surface).
    Runtime(String),
}

impl std::fmt::Display for ManagedFalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagedFalError::QueueOriginRequired => {
                write!(f, "Managed FAL queue origin is required")
            }
            ManagedFalError::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ManagedFalError {}

// ---------------------------------------------------------------------------
// import_fal_client — lazy import + lazy_deps integration
// ---------------------------------------------------------------------------

/// Outcome of a `tools.lazy_deps.ensure` call (mirror of the exceptions it
/// lets through).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FalEnsureError {
    /// `ImportError` from lazy_deps — swallowed by import_fal_client.
    ModuleMissing,
    /// Any non-ImportError — converted to an ImportError carrying this text.
    ReRaise(String),
}

/// The `tools.lazy_deps.ensure(name, prompt)` hook.
pub type EnsureHook = dyn Fn(&str, bool) -> Result<(), FalEnsureError> + Send + Sync;

/// The `import fal_client` machinery: returns the module handle or None when
/// the package is genuinely unavailable.
pub type FalClientProvider = dyn Fn() -> Option<Arc<FalClientModule>> + Send + Sync;

static ENSURE_HOOK: OnceLock<Mutex<Option<Arc<EnsureHook>>>> = OnceLock::new();
static FAL_CLIENT_PROVIDER: OnceLock<Mutex<Option<Arc<FalClientProvider>>>> = OnceLock::new();

/// Install (or clear) the lazy_deps.ensure hook. Seam — see module docs.
pub fn set_ensure_hook(hook: Option<Arc<EnsureHook>>) {
    let cell = ENSURE_HOOK.get_or_init(|| Mutex::new(None));
    *cell.lock().unwrap() = hook;
}

/// Install (or clear) the fal_client module provider. Seam — see module docs.
pub fn set_fal_client_provider(provider: Option<Arc<FalClientProvider>>) {
    let cell = FAL_CLIENT_PROVIDER.get_or_init(|| Mutex::new(None));
    *cell.lock().unwrap() = provider;
}

/// Upstream `import fal_client` ImportError analog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FalImportError(pub String);

impl std::fmt::Display for FalImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for FalImportError {}

fn ensure_hook() -> Option<Arc<EnsureHook>> {
    ENSURE_HOOK.get().and_then(|m| m.lock().ok()).and_then(|guard| guard.clone())
}

fn fal_client_provider() -> Option<Arc<FalClientProvider>> {
    FAL_CLIENT_PROVIDER.get().and_then(|m| m.lock().ok()).and_then(|guard| guard.clone())
}

/// Import `fal_client` and return the module reference.
///
/// Callers are responsible for caching the result on their own module
/// global — keeping per-module globals lets tests swap the provider per
/// module call site, mirroring the upstream monkey-patch contract.
///
/// Raises [`FalImportError`] if the package is genuinely unavailable.
pub fn import_fal_client() -> Result<Arc<FalClientModule>, FalImportError> {
    // tools.lazy_deps.ensure("image.fal", prompt=False) — ImportError from
    // lazy_deps itself is swallowed; any other exception becomes the
    // ImportError text.
    if let Some(hook) = ensure_hook() {
        match hook("image.fal", false) {
            Ok(()) => {}
            Err(FalEnsureError::ModuleMissing) => {}
            Err(FalEnsureError::ReRaise(msg)) => return Err(FalImportError(msg)),
        }
    }
    match fal_client_provider() {
        Some(provider) => provider()
            .ok_or_else(|| FalImportError("import of fal_client failed".to_string())),
        None => Err(FalImportError("import of fal_client failed".to_string())),
    }
}

// ---------------------------------------------------------------------------
// Small helpers used by both the managed client wrapper and _submit_fal_request
// ---------------------------------------------------------------------------

/// `str(queue_run_origin or "").strip().rstrip("/")`, plus the trailing
/// slash. Raises [`ManagedFalError::QueueOriginRequired`] when empty.
pub fn normalize_fal_queue_url_format(
    queue_run_origin: Option<&str>,
) -> Result<String, ManagedFalError> {
    let normalized = queue_run_origin.unwrap_or("").trim().trim_end_matches('/');
    if normalized.is_empty() {
        return Err(ManagedFalError::QueueOriginRequired);
    }
    Ok(format!("{normalized}/"))
}

/// Exception-shape probes used by `_extract_http_status`: first
/// `exc.response.status_code` (when a response is present), then
/// `exc.status_code`. Each accessor returns None when the attribute is
/// absent or not an int — mirroring the defensive getattr/isinstance checks.
pub trait HttpErrorShape {
    fn response_status_code(&self) -> Option<i64>;
    fn status_code(&self) -> Option<i64>;
}

/// Return an HTTP status code from httpx/fal exception shapes, else None.
pub fn extract_http_status<E: HttpErrorShape>(exc: &E) -> Option<i64> {
    if let Some(status) = exc.response_status_code() {
        return Some(status);
    }
    exc.status_code()
}

/// `urllib.parse.urlencode({"fal_webhook": value})` — i.e. quote_plus(value,
/// safe="") for this module's single-key usage: unreserved `A-Za-z0-9-._~`
/// kept, space → `+`, everything else `%XX` uppercase.
fn urlencode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Managed sync client primitives (the `fal_client` module attribute surface)
// ---------------------------------------------------------------------------

/// Instantiated `fal_client.SyncClient(key=...)` handle.
pub struct SyncClientHandle {
    /// `SyncClient._client` (required by the managed wrapper).
    pub http_client: Option<Arc<dyn HttpClientLike>>,
    /// `SyncClient.default_timeout` — upstream falls back to 120.0 when absent.
    pub default_timeout: Option<f64>,
}

/// `fal_client.SyncClient` class: `SyncClient(key=...)`.
pub type SyncClientFactory = dyn Fn(&str) -> SyncClientHandle + Send + Sync;

/// `add_hint_header(value, headers)` / `add_priority_header` /
/// `add_timeout_header`: the value is transported as a JSON value so the
/// caller's int/float/str identity is preserved for the header formatter.
pub type AddHeaderFn = dyn Fn(&Value, &mut HashMap<String, String>) + Send + Sync;

/// Arguments passed to `fal_client.client._maybe_retry_request(...)`.
pub struct FalSubmitRequest {
    pub http_client: Arc<dyn HttpClientLike>,
    pub method: String,
    pub url: String,
    pub json: Value,
    pub timeout: f64,
    pub headers: HashMap<String, String>,
}

/// A transporter response — `response.json()` is the body.
pub struct FalResponse {
    pub status_code: u16,
    pub body: Value,
}

impl FalResponse {
    pub fn json(&self) -> Value {
        self.body.clone()
    }
}

/// `fal_client.client._maybe_retry_request(http_client, "POST", url,
/// json=..., timeout=..., headers=...)`. Err(text) mirrors an exception the
/// upstream retry helper raises.
pub type RetryFn = dyn Fn(FalSubmitRequest) -> Result<FalResponse, String> + Send + Sync;

/// `fal_client.client._raise_for_status(response)`. Err(text) mirrors the
/// HTTPStatusError upstream raises on non-2xx.
pub type RaiseForStatusFn = dyn Fn(&FalResponse) -> Result<(), String> + Send + Sync;

/// `fal_client.client.SyncRequestHandle(request_id=..., response_url=...,
/// status_url=..., cancel_url=..., client=...)` construction.
pub type RequestHandleFactory = dyn Fn(
    String,
    String,
    String,
    String,
    Arc<dyn HttpClientLike>,
) -> RequestHandle + Send + Sync;

/// Result handle returned by [`ManagedFalSyncClient::submit`].
#[derive(Clone)]
pub struct RequestHandle {
    pub request_id: String,
    pub response_url: String,
    pub status_url: String,
    pub cancel_url: String,
    pub client: Arc<dyn HttpClientLike>,
}

/// The `fal_client.client` submodule attribute surface, probed like upstream
/// (missing attributes are None → the matching RuntimeError).
#[derive(Clone, Default)]
pub struct FalClientModuleClient {
    pub maybe_retry_request: Option<Arc<RetryFn>>,
    pub raise_for_status: Option<Arc<RaiseForStatusFn>>,
    pub request_handle_class: Option<Arc<RequestHandleFactory>>,
    pub add_hint_header: Option<Arc<AddHeaderFn>>,
    pub add_priority_header: Option<Arc<AddHeaderFn>>,
    pub add_timeout_header: Option<Arc<AddHeaderFn>>,
}

/// Mirrors the `fal_client` module reference handed to
/// [`ManagedFalSyncClient`] (the `SyncClient` class + the `client` module).
#[derive(Clone, Default)]
pub struct FalClientModule {
    pub sync_client: Option<Arc<SyncClientFactory>>,
    pub client: Option<FalClientModuleClient>,
}

// ---------------------------------------------------------------------------
// _ManagedFalSyncClient
// ---------------------------------------------------------------------------

/// Optional per-call parameters for [`ManagedFalSyncClient::submit`].
pub struct SubmitOptions<'a> {
    pub path: &'a str,
    pub hint: Option<&'a str>,
    pub webhook_url: Option<&'a str>,
    pub priority: Option<Value>,
    pub headers: Option<&'a HashMap<String, String>>,
    pub start_timeout: Option<Value>,
}

impl Default for SubmitOptions<'_> {
    fn default() -> Self {
        SubmitOptions {
            path: "",
            hint: None,
            webhook_url: None,
            priority: None,
            headers: None,
            start_timeout: None,
        }
    }
}

/// Small per-instance wrapper around fal_client.SyncClient for managed queue
/// hosts. Carries its own client-module references instead of reaching into
/// a module global, so callers stay in control of which module's fal_client
/// is in scope (matters for the test patches that swap the legacy module's
/// attribute).
pub struct ManagedFalSyncClient {
    queue_url_format: String,
    http_client: Arc<dyn HttpClientLike>,
    default_timeout: Option<f64>,
    maybe_retry_request: Arc<RetryFn>,
    raise_for_status: Arc<RaiseForStatusFn>,
    request_handle_class: Arc<RequestHandleFactory>,
    add_hint_header: Option<Arc<AddHeaderFn>>,
    add_priority_header: Option<Arc<AddHeaderFn>>,
    add_timeout_header: Option<Arc<AddHeaderFn>>,
}

impl ManagedFalSyncClient {
    pub fn new(
        fal_client: &FalClientModule,
        key: &str,
        queue_run_origin: Option<&str>,
    ) -> Result<Self, ManagedFalError> {
        let sync_client_class = fal_client.sync_client.clone().ok_or_else(|| {
            ManagedFalError::Runtime(
                "fal_client.SyncClient is required for managed FAL gateway mode".to_string(),
            )
        })?;

        let client_module = fal_client.client.clone().ok_or_else(|| {
            ManagedFalError::Runtime(
                "fal_client.client is required for managed FAL gateway mode".to_string(),
            )
        })?;

        let queue_url_format = normalize_fal_queue_url_format(queue_run_origin)?;

        // sync_client_class(key=key)
        let sync_client = (sync_client_class)(key);

        let http_client = sync_client.http_client.clone().ok_or_else(|| {
            ManagedFalError::Runtime(
                "fal_client.SyncClient._client is required for managed FAL gateway mode"
                    .to_string(),
            )
        })?;

        let maybe_retry_request = client_module.maybe_retry_request.clone();
        let raise_for_status = client_module.raise_for_status.clone();
        if maybe_retry_request.is_none() || raise_for_status.is_none() {
            return Err(ManagedFalError::Runtime(
                "fal_client.client request helpers are required for managed FAL gateway mode"
                    .to_string(),
            ));
        }

        let request_handle_class = client_module.request_handle_class.clone().ok_or_else(|| {
            ManagedFalError::Runtime(
                "fal_client.client.SyncRequestHandle is required for managed FAL gateway mode"
                    .to_string(),
            )
        })?;

        Ok(Self {
            queue_url_format,
            http_client,
            default_timeout: sync_client.default_timeout,
            maybe_retry_request: maybe_retry_request.unwrap(),
            raise_for_status: raise_for_status.unwrap(),
            request_handle_class,
            add_hint_header: client_module.add_hint_header.clone(),
            add_priority_header: client_module.add_priority_header.clone(),
            add_timeout_header: client_module.add_timeout_header.clone(),
        })
    }

    /// Parity-readable surface (upstream exposes these as public attributes).
    pub fn queue_url_format(&self) -> &str {
        &self.queue_url_format
    }

    /// Parity-readable surface (upstream exposes this as a public attribute).
    pub fn http_client(&self) -> Arc<dyn HttpClientLike> {
        self.http_client.clone()
    }

    /// Submit a queue run. Mirrors upstream `submit(application, arguments,
    /// *, path="", hint=None, webhook_url=None, priority=None, headers=None,
    /// start_timeout=None)`.
    pub fn submit(
        &self,
        application: &str,
        arguments: Value,
        options: SubmitOptions<'_>,
    ) -> Result<RequestHandle, ManagedFalError> {
        let mut url = format!("{}{}", self.queue_url_format, application);
        if !options.path.is_empty() {
            url.push('/');
            url.push_str(options.path.trim_start_matches('/'));
        }
        if let Some(webhook_url) = options.webhook_url {
            url.push_str("?fal_webhook=");
            url.push_str(&urlencode_query_value(webhook_url));
        }

        let mut request_headers = options.headers.cloned().unwrap_or_default();
        if let Some(hint) = options.hint {
            if let Some(add_hint_header) = &self.add_hint_header {
                add_hint_header(&Value::String(hint.to_string()), &mut request_headers);
            }
        }
        if let Some(priority) = &options.priority {
            let add_priority_header = self.add_priority_header.as_ref().ok_or_else(|| {
                ManagedFalError::Runtime(
                    "fal_client.client.add_priority_header is required for priority requests"
                        .to_string(),
                )
            })?;
            add_priority_header(priority, &mut request_headers);
        }
        if let Some(start_timeout) = &options.start_timeout {
            let add_timeout_header = self.add_timeout_header.as_ref().ok_or_else(|| {
                ManagedFalError::Runtime(
                    "fal_client.client.add_timeout_header is required for timeout requests"
                        .to_string(),
                )
            })?;
            add_timeout_header(start_timeout, &mut request_headers);
        }

        let timeout = self.default_timeout.unwrap_or(120.0);
        let response = (self.maybe_retry_request)(FalSubmitRequest {
            http_client: self.http_client.clone(),
            method: "POST".to_string(),
            url,
            json: arguments,
            timeout,
            headers: request_headers,
        })
        .map_err(ManagedFalError::Runtime)?;

        (self.raise_for_status)(&response).map_err(ManagedFalError::Runtime)?;

        let data = response.json();
        let string_field = |key: &str| -> Result<String, ManagedFalError> {
            data.get(key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    // Upstream would raise KeyError/TypeError here; the Rust
                    // seam surfaces the same failure as a message.
                    ManagedFalError::Runtime(format!(
                        "missing string field in FAL response: {key}"
                    ))
                })
        };
        let request_id = string_field("request_id")?;
        let response_url = string_field("response_url")?;
        let status_url = string_field("status_url")?;
        let cancel_url = string_field("cancel_url")?;

        let handle = (self.request_handle_class)(
            request_id,
            response_url,
            status_url,
            cancel_url,
            self.http_client.clone(),
        );
        Ok(handle)
    }
}
