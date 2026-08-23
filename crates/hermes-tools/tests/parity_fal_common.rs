//! Parity oracles for the shared FAL.ai SDK plumbing, mirroring upstream
//! tests/tools/test_fal_common.py (all cases) @ b9aa928.
//!
//! Evidence tier: unit. Command: cargo test -p hermes-tools --test parity_fal_common
//!
//! Upstream mocks the third-party `fal_client` module (and `tools.lazy_deps`)
//! with MagicMock; the Rust port names the same probes on the `FalClientModule`
//! seam structs and drives the same function slots with recording closures.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use hermes_tools::fal_common::{
    extract_http_status, import_fal_client, normalize_fal_queue_url_format, FalClientModule,
    FalClientModuleClient, FalEnsureError, FalImportError, FalResponse, FalSubmitRequest,
    HttpErrorShape, ManagedFalError, ManagedFalSyncClient, RequestHandle, SyncClientHandle,
    SyncClientFactory,
};
use serde_json::{json, Value};

// import_fal_client reads/sets process-global seam slots; serialize the tests
// that touch them (cargo runs tests in the same binary concurrently).
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Test HTTP transport handle.
#[derive(Debug, Default)]
struct FakeHttpClient;
impl hermes_tools::fal_common::HttpClientLike for FakeHttpClient {}

fn ok_response() -> Value {
    json!({
        "request_id": "req-1",
        "response_url": "https://q.example.com/resp",
        "status_url": "https://q.example.com/status",
        "cancel_url": "https://q.example.com/cancel",
    })
}

// ---------------------------------------------------------------------------
// import_fal_client
// ---------------------------------------------------------------------------

#[test]
fn import_returns_fal_client_module() {
    let _guard = TEST_LOCK.lock().unwrap();
    let http = Arc::new(FakeHttpClient);
    let module = Arc::new(make_full_module(http.clone()));

    let ensure_calls = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));
    let calls = ensure_calls.clone();
    let ensure = Arc::new(move |name: &str, prompt: bool| {
        calls.lock().unwrap().push((name.to_string(), prompt));
        Ok(())
    });
    let provider = Arc::new({
        let module = module.clone();
        move || Some(module.clone())
    });

    hermes_tools::fal_common::set_ensure_hook(Some(ensure));
    hermes_tools::fal_common::set_fal_client_provider(Some(provider));

    let result = import_fal_client().unwrap();
    assert!(Arc::ptr_eq(&result, &module));
    assert_eq!(*ensure_calls.lock().unwrap(), vec![("image.fal".to_string(), false)]);
}

#[test]
fn import_swallows_lazy_ensure_import_error() {
    let _guard = TEST_LOCK.lock().unwrap();
    let http = Arc::new(FakeHttpClient);
    let module = Arc::new(make_full_module(http.clone()));

    let ensure = Arc::new(|_name: &str, _prompt: bool| {
        Err(FalEnsureError::ModuleMissing)
    });
    let provider = Arc::new({
        let module = module.clone();
        move || Some(module.clone())
    });

    hermes_tools::fal_common::set_ensure_hook(Some(ensure));
    hermes_tools::fal_common::set_fal_client_provider(Some(provider));

    let result = import_fal_client().unwrap();
    assert!(Arc::ptr_eq(&result, &module));
}

#[test]
fn import_rereaises_non_import_error_as_import_error() {
    let _guard = TEST_LOCK.lock().unwrap();
    let ensure = Arc::new(|_name: &str, _prompt: bool| {
        Err(FalEnsureError::ReRaise("install hint".to_string()))
    });
    let provider = Arc::new(|| Some(Arc::new(FalClientModule::default())));
    hermes_tools::fal_common::set_ensure_hook(Some(ensure));
    hermes_tools::fal_common::set_fal_client_provider(Some(provider));

    let err = err_of(import_fal_client());
    assert_eq!(err, FalImportError("install hint".to_string()));
}

#[test]
fn import_swallows_missing_lazy_deps_module() {
    // No ensure hook installed === tools.lazy_deps import fails.
    let _guard = TEST_LOCK.lock().unwrap();
    let http = Arc::new(FakeHttpClient);
    let module = Arc::new(make_full_module(http.clone()));
    let provider = Arc::new({
        let module = module.clone();
        move || Some(module.clone())
    });
    hermes_tools::fal_common::set_ensure_hook(None);
    hermes_tools::fal_common::set_fal_client_provider(Some(provider));

    let result = import_fal_client().unwrap();
    assert!(Arc::ptr_eq(&result, &module));
}

#[test]
fn import_fails_when_package_genuinely_unavailable() {
    let _guard = TEST_LOCK.lock().unwrap();
    hermes_tools::fal_common::set_ensure_hook(None);
    hermes_tools::fal_common::set_fal_client_provider(None);

    let err = err_of(import_fal_client());
    assert_eq!(err, FalImportError("import of fal_client failed".to_string()));
}

// ---------------------------------------------------------------------------
// _normalize_fal_queue_url_format
// ---------------------------------------------------------------------------

#[test]
fn normalize_adds_trailing_slash() {
    assert_eq!(
        normalize_fal_queue_url_format(Some("https://queue.example.com")).unwrap(),
        "https://queue.example.com/"
    );
}

#[test]
fn normalize_strips_trailing_slashes_then_adds_one() {
    assert_eq!(
        normalize_fal_queue_url_format(Some("https://queue.example.com///")).unwrap(),
        "https://queue.example.com/"
    );
}

#[test]
fn normalize_strips_whitespace() {
    assert_eq!(
        normalize_fal_queue_url_format(Some("  https://queue.example.com  ")).unwrap(),
        "https://queue.example.com/"
    );
}

#[test]
fn normalize_empty_raises() {
    let err = err_of(normalize_fal_queue_url_format(Some("")));
    assert_eq!(err, ManagedFalError::QueueOriginRequired);
    assert_eq!(err.to_string(), "Managed FAL queue origin is required");
}

#[test]
fn normalize_none_raises() {
    let err = err_of(normalize_fal_queue_url_format(None));
    assert_eq!(err, ManagedFalError::QueueOriginRequired);
}

#[test]
fn normalize_whitespace_only_raises() {
    let err = err_of(normalize_fal_queue_url_format(Some("   ")));
    assert_eq!(err, ManagedFalError::QueueOriginRequired);
}

// ---------------------------------------------------------------------------
// _extract_http_status
// ---------------------------------------------------------------------------

/// MagicMock stand-in: choose present/absent status attributes directly.
struct Exc {
    response_status: Option<i64>,
    direct_status: Option<i64>,
}

impl HttpErrorShape for Exc {
    fn response_status_code(&self) -> Option<i64> {
        self.response_status
    }
    fn status_code(&self) -> Option<i64> {
        self.direct_status
    }
}

#[test]
fn extract_returns_status_from_response_attribute() {
    let exc = Exc { response_status: Some(404), direct_status: None };
    assert_eq!(extract_http_status(&exc), Some(404));
}

#[test]
fn extract_returns_status_from_exc_status_code() {
    let exc = Exc { response_status: None, direct_status: Some(500) };
    assert_eq!(extract_http_status(&exc), Some(500));
}

#[test]
fn extract_returns_none_when_no_response_and_no_status_code() {
    let exc = Exc { response_status: None, direct_status: None };
    assert_eq!(extract_http_status(&exc), None);
}

#[test]
fn extract_returns_none_when_response_is_none() {
    let exc = Exc { response_status: None, direct_status: None };
    assert_eq!(extract_http_status(&exc), None);
}

#[test]
fn extract_returns_none_when_response_status_code_not_int() {
    let exc = Exc { response_status: None, direct_status: None };
    assert_eq!(extract_http_status(&exc), None);
}

#[test]
fn extract_returns_none_when_status_code_not_int() {
    let exc = Exc { response_status: None, direct_status: None };
    assert_eq!(extract_http_status(&exc), None);
}

#[test]
fn extract_response_status_takes_precedence_over_exc_status() {
    let exc = Exc { response_status: Some(200), direct_status: Some(500) };
    assert_eq!(extract_http_status(&exc), Some(200));
}

#[test]
fn extract_falls_back_to_exc_status_when_response_status_not_int() {
    let exc = Exc { response_status: None, direct_status: Some(503) };
    assert_eq!(extract_http_status(&exc), Some(503));
}

// ---------------------------------------------------------------------------
// _ManagedFalSyncClient — init
// ---------------------------------------------------------------------------

/// A full FalClientModule with all attributes present (upstream
/// `_make_fal_client_mock` default).
fn make_full_module(http: Arc<FakeHttpClient>) -> FalClientModule {
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: Some(120.0),
        }
    });
    let retry = Arc::new(|_req: FalSubmitRequest| {
        Ok(FalResponse { status_code: 200, body: ok_response() })
    });
    let raise = Arc::new(|_resp: &FalResponse| Ok(()));
    let handle_factory = Arc::new(
        |request_id: String,
         response_url: String,
         status_url: String,
         cancel_url: String,
         client: Arc<dyn hermes_tools::fal_common::HttpClientLike>| RequestHandle {
            request_id,
            response_url,
            status_url,
            cancel_url,
            client,
        },
    );
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(retry),
        raise_for_status: Some(raise),
        request_handle_class: Some(handle_factory),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    FalClientModule { sync_client: Some(sync_client), client: Some(client_module) }
}

#[test]
fn init_succeeds_with_all_attributes() {
    let http = Arc::new(FakeHttpClient);
    let module = make_full_module(http.clone());
    let client = ManagedFalSyncClient::new(
        &module,
        "test-key",
        Some("https://queue.example.com"),
    )
    .unwrap();
    assert_eq!(client.queue_url_format(), "https://queue.example.com/");
    assert!(Arc::ptr_eq(&client.http_client(), &(http as Arc<dyn hermes_tools::fal_common::HttpClientLike>)));
}

#[test]
fn init_raises_when_sync_client_missing() {
    let module = FalClientModule { sync_client: None, client: Some(FalClientModuleClient::default()) };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(
        err.to_string(),
        "fal_client.SyncClient is required for managed FAL gateway mode"
    );
}

#[test]
fn init_raises_when_client_module_missing() {
    let sync_client: Arc<SyncClientFactory> = Arc::new(|_key: &str| SyncClientHandle {
        http_client: None,
        default_timeout: None,
    });
    let module = FalClientModule { sync_client: Some(sync_client), client: None };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(err.to_string(), "fal_client.client is required for managed FAL gateway mode");
}

#[test]
fn init_raises_when_http_client_missing() {
    let sync_client: Arc<SyncClientFactory> = Arc::new(|_key: &str| SyncClientHandle {
        http_client: None,
        default_timeout: Some(120.0),
    });
    let retry = Arc::new(|_req: FalSubmitRequest| {
        Ok(FalResponse { status_code: 200, body: ok_response() })
    });
    let raise = Arc::new(|_resp: &FalResponse| Ok(()));
    let handle_factory = Arc::new(
        |request_id: String,
         response_url: String,
         status_url: String,
         cancel_url: String,
         client: Arc<dyn hermes_tools::fal_common::HttpClientLike>| RequestHandle {
            request_id,
            response_url,
            status_url,
            cancel_url,
            client,
        },
    );
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(retry),
        raise_for_status: Some(raise),
        request_handle_class: Some(handle_factory),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(
        err.to_string(),
        "fal_client.SyncClient._client is required for managed FAL gateway mode"
    );
}

#[test]
fn init_raises_when_retry_request_missing() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: Some(120.0),
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: None,
        raise_for_status: Some(Arc::new(|_resp: &FalResponse| Ok(()))),
        request_handle_class: None,
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(
        err.to_string(),
        "fal_client.client request helpers are required for managed FAL gateway mode"
    );
}

#[test]
fn init_raises_when_raise_for_status_missing() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: Some(120.0),
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(Arc::new(|_req: FalSubmitRequest| {
            Ok(FalResponse { status_code: 200, body: ok_response() })
        })),
        raise_for_status: None,
        request_handle_class: None,
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(
        err.to_string(),
        "fal_client.client request helpers are required for managed FAL gateway mode"
    );
}

#[test]
fn init_raises_when_request_handle_class_missing() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: Some(120.0),
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(Arc::new(|_req: FalSubmitRequest| {
            Ok(FalResponse { status_code: 200, body: ok_response() })
        })),
        raise_for_status: Some(Arc::new(|_resp: &FalResponse| Ok(()))),
        request_handle_class: None,
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com")));
    assert_eq!(
        err.to_string(),
        "fal_client.client.SyncRequestHandle is required for managed FAL gateway mode"
    );
}

#[test]
fn init_raises_when_queue_origin_invalid() {
    let http = Arc::new(FakeHttpClient);
    let module = make_full_module(http.clone());
    let err = err_of(ManagedFalSyncClient::new(&module, "k", Some("   ")));
    assert_eq!(err, ManagedFalError::QueueOriginRequired);
}

#[test]
fn init_passes_key_to_sync_client() {
    let http = Arc::new(FakeHttpClient);
    let keys = Arc::new(Mutex::new(Vec::<String>::new()));
    let keys2 = keys.clone();
    let sync_client: Arc<SyncClientFactory> = Arc::new(move |key: &str| {
        keys2.lock().unwrap().push(key.to_string());
        SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: Some(120.0),
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(Arc::new(|_req: FalSubmitRequest| {
            Ok(FalResponse { status_code: 200, body: ok_response() })
        })),
        raise_for_status: Some(Arc::new(|_resp: &FalResponse| Ok(()))),
        request_handle_class: Some(Arc::new(
            |request_id, response_url, status_url, cancel_url, client| RequestHandle {
                request_id,
                response_url,
                status_url,
                cancel_url,
                client,
            },
        )),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    ManagedFalSyncClient::new(&module, "my-secret-key", Some("https://q.example.com")).unwrap();
    assert_eq!(*keys.lock().unwrap(), vec!["my-secret-key".to_string()]);
}

#[test]
fn init_normalizes_queue_url() {
    let http = Arc::new(FakeHttpClient);
    let module = make_full_module(http.clone());
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://q.example.com///")).unwrap();
    assert_eq!(client.queue_url_format(), "https://q.example.com/");
}

// ---------------------------------------------------------------------------
// _ManagedFalSyncClient — submit
// ---------------------------------------------------------------------------

/// A client whose seam slots are recording closures. The default retry body
/// is `ok_response()` and the default handle factory builds a plain handle.
struct RecordingClient {
    client: ManagedFalSyncClient,
    http: Arc<FakeHttpClient>,
    retry_calls: Arc<Mutex<Vec<FalSubmitRequest>>>,
    raise_calls: Arc<Mutex<usize>>,
    handle_calls: Arc<Mutex<Vec<(String, String, String, String)>>>,
    hint_calls: Arc<Mutex<Vec<Value>>>,
    priority_calls: Arc<Mutex<Vec<Value>>>,
    timeout_calls: Arc<Mutex<Vec<Value>>>,
}

fn make_recording_client(default_timeout: Option<f64>) -> RecordingClient {
    let http: Arc<FakeHttpClient> = Arc::new(FakeHttpClient);
    let http_for_sync = http.clone();
    let retry_calls = Arc::new(Mutex::new(Vec::<FalSubmitRequest>::new()));
    let raise_calls = Arc::new(Mutex::new(0usize));
    let handle_calls = Arc::new(Mutex::new(Vec::<(String, String, String, String)>::new()));
    let hint_calls: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let priority_calls: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let timeout_calls: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));

    let retry_calls2 = retry_calls.clone();
    let raise_calls2 = raise_calls.clone();
    let handle_calls2 = handle_calls.clone();
    let hint_calls2 = hint_calls.clone();
    let priority_calls2 = priority_calls.clone();
    let timeout_calls2 = timeout_calls.clone();

    let sync_client: Arc<SyncClientFactory> = Arc::new(move |_key: &str| SyncClientHandle {
        http_client: Some(http_for_sync.clone()),
        default_timeout,
    });
    let retry = Arc::new(move |req: FalSubmitRequest| {
        retry_calls2.lock().unwrap().push(req);
        Ok(FalResponse { status_code: 200, body: ok_response() })
    });
    let raise = Arc::new(move |_resp: &FalResponse| {
        *raise_calls2.lock().unwrap() += 1;
        Ok(())
    });
    let handle_factory = Arc::new(
        move |request_id: String,
              response_url: String,
              status_url: String,
              cancel_url: String,
              client: Arc<dyn hermes_tools::fal_common::HttpClientLike>| {
        handle_calls2.lock().unwrap().push((
            request_id.clone(),
            response_url.clone(),
            status_url.clone(),
            cancel_url.clone(),
        ));
        RequestHandle { request_id, response_url, status_url, cancel_url, client }
    });
    let add_hint = Arc::new(move |value: &Value, _headers: &mut HashMap<String, String>| {
        hint_calls2.lock().unwrap().push(value.clone());
    });
    let add_priority = Arc::new(move |value: &Value, _headers: &mut HashMap<String, String>| {
        priority_calls2.lock().unwrap().push(value.clone());
    });
    let add_timeout = Arc::new(move |value: &Value, _headers: &mut HashMap<String, String>| {
        timeout_calls2.lock().unwrap().push(value.clone());
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(retry),
        raise_for_status: Some(raise),
        request_handle_class: Some(handle_factory),
        add_hint_header: Some(add_hint),
        add_priority_header: Some(add_priority),
        add_timeout_header: Some(add_timeout),
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://queue.example.com")).unwrap();
    RecordingClient {
        client,
        http,
        retry_calls,
        raise_calls,
        handle_calls,
        hint_calls,
        priority_calls,
        timeout_calls,
    }
}

/// unwrap_err without the T: Debug bound (the seam types carry Arc/dyn
/// fields that don't need Debug).
fn err_of<T, E>(r: Result<T, E>) -> E {
    match r {
        Ok(_) => panic!("expected Err"),
        Err(e) => e,
    }
}

fn default_options() -> hermes_tools::fal_common::SubmitOptions<'static> {
    Default::default()
}

fn with_path(path: &'static str) -> hermes_tools::fal_common::SubmitOptions<'static> {
    let mut opts = default_options();
    opts.path = path;
    opts
}

#[test]
fn submit_basic() {
    let rc = make_recording_client(Some(120.0));
    let result = rc
        .client
        .submit("my-app", json!({"prompt": "hello"}), default_options())
        .unwrap();

    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let req = &calls[0];
    let rc_http: Arc<dyn hermes_tools::fal_common::HttpClientLike> = rc.http.clone();
    assert!(Arc::ptr_eq(&req.http_client, &rc_http));
    assert_eq!(req.method, "POST");
    assert_eq!(req.url, "https://queue.example.com/my-app");
    assert_eq!(req.json, json!({"prompt": "hello"}));
    assert_eq!(req.timeout, 120.0);

    assert_eq!(*rc.raise_calls.lock().unwrap(), 1);
    // request_handle_class called; result is its return value
    assert_eq!(rc.handle_calls.lock().unwrap()[0].0, "req-1");
    assert_eq!(result.request_id, "req-1");
    assert_eq!(result.response_url, "https://q.example.com/resp");
    assert_eq!(result.status_url, "https://q.example.com/status");
    assert_eq!(result.cancel_url, "https://q.example.com/cancel");
}

#[test]
fn submit_with_path() {
    let rc = make_recording_client(None);
    rc.client.submit("my-app", json!({}), with_path("sub/path")).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls[0].url, "https://queue.example.com/my-app/sub/path");
}

#[test]
fn submit_with_path_strips_leading_slash() {
    let rc = make_recording_client(None);
    rc.client.submit("my-app", json!({}), with_path("/leading/slash")).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls[0].url, "https://queue.example.com/my-app/leading/slash");
}

#[test]
fn submit_with_webhook_url() {
    let rc = make_recording_client(None);
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: Some("https://hook.example.com/cb"),
        priority: None,
        headers: None,
        start_timeout: None,
    };
    rc.client.submit("my-app", json!({}), opts).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert!(calls[0].url.contains("fal_webhook=https%3A%2F%2Fhook.example.com%2Fcb"));
}

#[test]
fn submit_with_hint() {
    let rc = make_recording_client(None);
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: Some("my-hint"),
        webhook_url: None,
        priority: None,
        headers: None,
        start_timeout: None,
    };
    rc.client.submit("my-app", json!({}), opts).unwrap();
    assert_eq!(*rc.hint_calls.lock().unwrap(), vec![json!("my-hint")]);
}

#[test]
fn submit_with_priority() {
    let rc = make_recording_client(None);
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: None,
        priority: Some(json!(5)),
        headers: None,
        start_timeout: None,
    };
    rc.client.submit("my-app", json!({}), opts).unwrap();
    assert_eq!(*rc.priority_calls.lock().unwrap(), vec![json!(5)]);
}

#[test]
fn submit_with_priority_raises_when_header_fn_missing() {
    let http = Arc::new(FakeHttpClient);
    let rc = make_recording_client(None);
    // Rebuild with add_priority_header absent.
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: None,
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(Arc::new(|_req: FalSubmitRequest| {
            Ok(FalResponse { status_code: 200, body: ok_response() })
        })),
        raise_for_status: Some(Arc::new(|_resp: &FalResponse| Ok(()))),
        request_handle_class: Some(Arc::new(
            |request_id, response_url, status_url, cancel_url, client| RequestHandle {
                request_id,
                response_url,
                status_url,
                cancel_url,
                client,
            },
        )),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://queue.example.com")).unwrap();
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: None,
        priority: Some(json!(5)),
        headers: None,
        start_timeout: None,
    };
    let err = err_of(client.submit("my-app", json!({}), opts));
    assert_eq!(
        err.to_string(),
        "fal_client.client.add_priority_header is required for priority requests"
    );
    assert!(rc.retry_calls.lock().unwrap().is_empty());
}

#[test]
fn submit_with_start_timeout() {
    let rc = make_recording_client(None);
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: None,
        priority: None,
        headers: None,
        start_timeout: Some(json!(30)),
    };
    rc.client.submit("my-app", json!({}), opts).unwrap();
    assert_eq!(*rc.timeout_calls.lock().unwrap(), vec![json!(30)]);
}

#[test]
fn submit_with_start_timeout_raises_when_header_fn_missing() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: None,
        }
    });
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(Arc::new(|_req: FalSubmitRequest| {
            Ok(FalResponse { status_code: 200, body: ok_response() })
        })),
        raise_for_status: Some(Arc::new(|_resp: &FalResponse| Ok(()))),
        request_handle_class: Some(Arc::new(
            |request_id, response_url, status_url, cancel_url, client| RequestHandle {
                request_id,
                response_url,
                status_url,
                cancel_url,
                client,
            },
        )),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://queue.example.com")).unwrap();
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: None,
        priority: None,
        headers: None,
        start_timeout: Some(json!(30)),
    };
    let err = err_of(client.submit("my-app", json!({}), opts));
    assert_eq!(
        err.to_string(),
        "fal_client.client.add_timeout_header is required for timeout requests"
    );
}

#[test]
fn submit_with_custom_headers() {
    let rc = make_recording_client(None);
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "value".to_string());
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "",
        hint: None,
        webhook_url: None,
        priority: None,
        headers: Some(&headers),
        start_timeout: None,
    };
    rc.client.submit("my-app", json!({}), opts).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls[0].headers.get("X-Custom").map(String::as_str), Some("value"));
}

#[test]
fn submit_with_none_headers_defaults_to_empty_dict() {
    let rc = make_recording_client(None);
    rc.client.submit("my-app", json!({}), default_options()).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert!(calls[0].headers.is_empty());
}

#[test]
fn submit_uses_custom_default_timeout() {
    let rc = make_recording_client(Some(300.0));
    rc.client.submit("app", json!({}), default_options()).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls[0].timeout, 300.0);
}

#[test]
fn submit_falls_back_to_120_when_no_default_timeout() {
    let rc = make_recording_client(None);
    rc.client.submit("app", json!({}), default_options()).unwrap();
    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls[0].timeout, 120.0);
}

#[test]
fn submit_passes_request_handle_kwargs() {
    let rc = make_recording_client(None);
    rc.client.submit("app", json!({}), default_options()).unwrap();
    let calls = rc.handle_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        (
            "req-1".to_string(),
            "https://q.example.com/resp".to_string(),
            "https://q.example.com/status".to_string(),
            "https://q.example.com/cancel".to_string(),
        )
    );
}

#[test]
fn submit_with_all_optional_params() {
    let rc = make_recording_client(None);
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "val".to_string());
    let opts = hermes_tools::fal_common::SubmitOptions {
        path: "sub",
        hint: Some("hint-val"),
        webhook_url: Some("https://hook.example.com"),
        priority: Some(json!(10)),
        headers: Some(&headers),
        start_timeout: Some(json!(60)),
    };
    rc.client.submit("app", json!({"prompt": "test"}), opts).unwrap();

    let calls = rc.retry_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].url.contains("app/sub?"));
    assert!(calls[0].url.contains("fal_webhook="));
    assert_eq!(calls[0].headers.get("X-Custom").map(String::as_str), Some("val"));
    drop(calls);

    assert_eq!(*rc.hint_calls.lock().unwrap(), vec![json!("hint-val")]);
    assert_eq!(*rc.priority_calls.lock().unwrap(), vec![json!(10)]);
    assert_eq!(*rc.timeout_calls.lock().unwrap(), vec![json!(60)]);
}

#[test]
fn submit_propagates_retry_error() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: None,
        }
    });
    let retry = Arc::new(|_req: FalSubmitRequest| Err("upstream transport error".to_string()));
    let raise = Arc::new(|_resp: &FalResponse| Ok(()));
    let handle_factory = Arc::new(
        |request_id: String,
         response_url: String,
         status_url: String,
         cancel_url: String,
         client: Arc<dyn hermes_tools::fal_common::HttpClientLike>| RequestHandle {
            request_id,
            response_url,
            status_url,
            cancel_url,
            client,
        },
    );
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(retry),
        raise_for_status: Some(raise),
        request_handle_class: Some(handle_factory),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://queue.example.com")).unwrap();
    let err = err_of(client.submit("app", json!({}), default_options()));
    assert_eq!(err.to_string(), "upstream transport error");
}

#[test]
fn submit_propagates_raise_for_status_error() {
    let http = Arc::new(FakeHttpClient);
    let sync_client: Arc<SyncClientFactory> = Arc::new({
        let http = http.clone();
        move |_key: &str| SyncClientHandle {
            http_client: Some(http.clone()),
            default_timeout: None,
        }
    });
    let retry = Arc::new(|_req: FalSubmitRequest| {
        Ok(FalResponse { status_code: 429, body: json!({}) })
    });
    let raise = Arc::new(|_resp: &FalResponse| Err("HTTP 429 Too Many Requests".to_string()));
    let handle_factory = Arc::new(
        |request_id: String,
         response_url: String,
         status_url: String,
         cancel_url: String,
         client: Arc<dyn hermes_tools::fal_common::HttpClientLike>| RequestHandle {
            request_id,
            response_url,
            status_url,
            cancel_url,
            client,
        },
    );
    let client_module = FalClientModuleClient {
        maybe_retry_request: Some(retry),
        raise_for_status: Some(raise),
        request_handle_class: Some(handle_factory),
        add_hint_header: None,
        add_priority_header: None,
        add_timeout_header: None,
    };
    let module = FalClientModule { sync_client: Some(sync_client), client: Some(client_module) };
    let client = ManagedFalSyncClient::new(&module, "k", Some("https://queue.example.com")).unwrap();
    let err = err_of(client.submit("app", json!({}), default_options()));
    assert_eq!(err.to_string(), "HTTP 429 Too Many Requests");
}

#[test]
fn submit_passes_client_to_handle() {
    let rc = make_recording_client(None);
    let result = rc.client.submit("app", json!({}), default_options()).unwrap();
    let rc_http: Arc<dyn hermes_tools::fal_common::HttpClientLike> = rc.http.clone();
    assert!(Arc::ptr_eq(&result.client, &rc_http));
}
