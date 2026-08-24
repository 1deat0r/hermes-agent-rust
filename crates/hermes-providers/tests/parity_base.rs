//! Parity oracles for `providers/base.py`, mirroring the pinned upstream
//! provider profile and model-catalog behavior at b9aa928.
//!
//! Tier: mock/unit. The HTTP tests use loopback listeners as the upstream
//! tests use local HTTPServer instances.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use hermes_providers::base::{
    profile_user_agent, FixedTemperature, ModelsFetchMode, ProviderProfile, OMIT_TEMPERATURE,
};
use serde_json::{json, Map};

fn response(status: &str, body: &str, extra_headers: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}",
        body.len()
    )
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 1024];
    loop {
        let n = stream.read(&mut buf).expect("read request");
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn spawn_server(
    connections: usize,
    handler: impl Fn(&str) -> String + Send + Sync + 'static,
) -> (String, Arc<Mutex<Vec<String>>>, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let address = listener.local_addr().expect("server address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let handler = Arc::new(handler);
    let thread = thread::spawn(move || {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().expect("accept loopback request");
            let request = read_request(&mut stream);
            captured.lock().unwrap().push(request.clone());
            stream
                .write_all(handler(&request).as_bytes())
                .expect("write loopback response");
        }
    });
    (format!("http://{address}"), requests, thread)
}

fn spawn_same_origin_redirect_server() -> (String, Arc<Mutex<Vec<String>>>, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let address = listener.local_addr().expect("server address");
    let base_url = format!("http://{address}");
    let redirect_base_url = base_url.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let thread = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept loopback request");
            let request = read_request(&mut stream);
            captured.lock().unwrap().push(request.clone());
            let body = if request.starts_with("GET /models ") {
                String::new()
            } else {
                r#"{"data":[{"id":"redirected-model"}]}"#.into()
            };
            let reply = if request.starts_with("GET /models ") {
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: {redirect_base_url}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
            } else {
                response("200 OK", &body, "")
            };
            stream.write_all(reply.as_bytes()).expect("write response");
        }
    });
    (base_url, requests, thread)
}

#[test]
fn dataclass_defaults_match_upstream() {
    let profile = ProviderProfile::new("test");
    assert_eq!(profile.name, "test");
    assert_eq!(profile.api_mode, "chat_completions");
    assert!(profile.aliases.is_empty());
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert!(profile.env_vars.is_empty());
    assert!(profile.base_url.is_empty());
    assert!(profile.models_url.is_empty());
    assert_eq!(profile.auth_type, "api_key");
    assert!(profile.supports_health_check);
    assert!(!profile.supports_vision);
    assert!(profile.supports_vision_tool_messages);
    assert!(!profile.supports_prompt_cache_key);
    assert!(profile.fallback_models.is_empty());
    assert!(profile.hostname.is_empty());
    assert!(profile.default_headers.is_empty());
    assert_eq!(profile.fixed_temperature, FixedTemperature::CallerDefault);
    assert_eq!(OMIT_TEMPERATURE, FixedTemperature::Omit);
    assert_eq!(profile.default_max_tokens, None);
    assert!(profile.default_aux_model.is_empty());
    assert!(!profile.actual_catalog);
    assert!(!profile.models_fetch_disabled);
    assert_eq!(profile.models_fetch_mode, ModelsFetchMode::Standard);
    assert!(!profile.gemini_thinking);
    assert!(!profile.vertex_thinking);
    assert!(!profile.deepinfra_vision);
    assert!(!profile.deepseek_reasoning);
    assert!(!profile.nous_portal);
    assert!(!profile.copilot_reasoning);
    assert!(!profile.reasoning_passthrough);
}

#[test]
fn hostname_prefers_explicit_then_derives_from_base_url() {
    let mut explicit = ProviderProfile::new("test");
    explicit.hostname = "explicit.example".into();
    explicit.base_url = "https://derived.example/v1".into();
    assert_eq!(explicit.get_hostname(), "explicit.example");

    let mut derived = ProviderProfile::new("test");
    derived.base_url = "https://api.example.test/v1".into();
    assert_eq!(derived.get_hostname(), "api.example.test");

    assert_eq!(ProviderProfile::new("test").get_hostname(), "");
}

#[test]
fn default_hooks_are_passthrough_or_empty() {
    let profile = ProviderProfile::new("test");
    let messages = vec![json!({"role": "user", "content": "hello"})];
    assert_eq!(profile.prepare_messages(&messages), messages);

    let context = Map::new();
    assert!(profile.build_extra_body(None, &context).is_empty());
    assert_eq!(
        profile.build_api_kwargs_extras(None, &context),
        (Map::new(), Map::new())
    );
    assert_eq!(profile.default_vision_model(), None);
    assert_eq!(profile.get_max_tokens(None), None);
}

#[test]
fn get_max_tokens_returns_static_profile_cap() {
    let mut profile = ProviderProfile::new("test");
    profile.default_max_tokens = Some(16_384);
    assert_eq!(profile.get_max_tokens(Some("model-a")), Some(16_384));
    assert_eq!(profile.get_max_tokens(None), Some(16_384));
}

#[test]
fn fetch_models_uses_caller_base_url_override() {
    let (server_url, requests, thread) = spawn_server(1, |_request| {
        response("200 OK", r#"{"data":[{"id":"proxy-model-a"}]}"#, "")
    });
    let mut profile = ProviderProfile::new("test");
    profile.base_url = "http://127.0.0.1:1".into();
    let models = profile.fetch_models(Some("test-key"), Some(&server_url), 8.0);
    thread.join().unwrap();
    assert_eq!(models, Some(vec!["proxy-model-a".into()]));
    let lower = requests
        .lock()
        .unwrap()
        .first()
        .cloned()
        .unwrap()
        .to_ascii_lowercase();
    assert!(lower.contains("authorization: bearer test-key"));
    assert!(lower.contains("accept: application/json"));
    assert!(lower.contains("user-agent: hermes-cli"));
}

#[test]
fn fetch_models_prefers_explicit_models_url() {
    let (server_url, _, thread) = spawn_server(1, |_request| {
        response("200 OK", r#"[{"id":"explicit-model"}]"#, "")
    });
    let mut profile = ProviderProfile::new("test");
    profile.base_url = "http://127.0.0.1:1".into();
    profile.models_url = server_url;
    let models = profile.fetch_models(None, Some("http://127.0.0.1:2"), 8.0);
    thread.join().unwrap();
    assert_eq!(models, Some(vec!["explicit-model".into()]));
}

#[test]
fn fetch_models_fails_open_for_missing_endpoint_or_malformed_payload() {
    let profile = ProviderProfile::new("test");
    assert_eq!(profile.fetch_models(None, None, 0.1), None);

    let (server_url, _, thread) = spawn_server(1, |_request| {
        response("200 OK", r#"{"unexpected":true}"#, "")
    });
    let models = profile.fetch_models(None, Some(&server_url), 8.0);
    thread.join().unwrap();
    assert_eq!(models, Some(Vec::new()));
}

#[test]
fn cross_origin_redirect_strips_credential_headers() {
    let (target_url, target_requests, target_thread) = spawn_server(1, |_request| {
        response("200 OK", r#"{"data":[{"id":"redirected-model"}]}"#, "")
    });
    let location = format!(
        "http://localhost:{}/redirected",
        target_url.rsplit(':').next().unwrap()
    );
    let (source_url, _, source_thread) = spawn_server(1, move |_request| {
        response("302 Found", "", &format!("Location: {location}\r\n"))
    });

    let mut profile = ProviderProfile::new("test");
    profile.base_url = source_url;
    profile
        .default_headers
        .insert("x-api-key".into(), "default-header-secret".into());
    let models = profile.fetch_models(Some("bearer-secret"), None, 8.0);

    source_thread.join().unwrap();
    target_thread.join().unwrap();
    let request = target_requests.lock().unwrap().first().cloned().unwrap();
    let lower = request.to_ascii_lowercase();
    assert_eq!(models, Some(vec!["redirected-model".into()]));
    assert!(!lower.contains("authorization:"));
    assert!(!lower.contains("x-api-key:"));
}

#[test]
fn same_origin_redirect_keeps_credential_headers() {
    let (server_url, requests, thread) = spawn_same_origin_redirect_server();
    let mut profile = ProviderProfile::new("test");
    profile.base_url = server_url;
    profile
        .default_headers
        .insert("x-api-key".into(), "default-header-secret".into());
    let models = profile.fetch_models(Some("bearer-secret"), None, 8.0);

    thread.join().unwrap();
    let request = requests.lock().unwrap().last().cloned().unwrap();
    let lower = request.to_ascii_lowercase();
    assert_eq!(models, Some(vec!["redirected-model".into()]));
    assert!(lower.contains("authorization: bearer bearer-secret"));
    assert!(lower.contains("x-api-key: default-header-secret"));
    assert_eq!(profile_user_agent(), "hermes-cli");
}
