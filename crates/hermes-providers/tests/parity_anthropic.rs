//! Source-derived parity oracle for
//! `plugins/model-providers/anthropic/__init__.py` @ b9aa928.
//!
//! No dedicated upstream profile test module exists for Anthropic, so the
//! pinned source is the oracle. The loopback HTTP cases are mock-tier tests;
//! the declarative profile assertions are unit-tier tests.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use hermes_providers::registry::{get_provider_profile, reset_registry_for_tests};
use hermes_providers::ModelsFetchMode;

static ANTHROPIC_TEST_LOCK: Mutex<()> = Mutex::new(());

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer).expect("read request");
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn spawn_server(body: &'static str) -> (String, Arc<Mutex<Vec<String>>>, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let address = listener.local_addr().expect("server address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept loopback request");
        let request = read_request(&mut stream);
        captured.lock().unwrap().push(request);
        stream
            .write_all(response("200 OK", body).as_bytes())
            .expect("write loopback response");
    });
    (format!("http://{address}"), requests, thread)
}

#[test]
fn anthropic_profile_fields_and_empty_key_short_circuit_match_upstream() {
    let _guard = ANTHROPIC_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("anthropic").expect("Anthropic profile must be registered");

    assert_eq!(profile.name, "anthropic");
    assert_eq!(profile.aliases, ["claude", "claude-oauth", "claude-code"]);
    assert_eq!(profile.api_mode, "anthropic_messages");
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert_eq!(
        profile.env_vars,
        [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN"
        ]
    );
    assert_eq!(profile.base_url, "https://api.anthropic.com");
    assert!(profile.models_url.is_empty());
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.models_fetch_mode, ModelsFetchMode::Anthropic);
    assert!(profile.default_headers.is_empty());
    assert!(profile.fallback_models.is_empty());
    assert_eq!(profile.default_aux_model, "claude-haiku-4-5-20251001");
    assert!(!profile.models_fetch_disabled);
    for alias in ["claude", "claude-oauth", "claude-code"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "anthropic");
    }

    // PARITY: AnthropicProfile.fetch_models() returns before opening a URL
    // when api_key is None or an empty string, even if base_url is supplied.
    assert_eq!(
        profile.fetch_models(None, Some("http://127.0.0.1:1"), 8.0),
        None
    );
    assert_eq!(
        profile.fetch_models(Some(""), Some("http://127.0.0.1:1"), 8.0),
        None
    );
}

#[test]
fn anthropic_model_fetch_uses_native_headers_and_filters_ids() {
    let _guard = ANTHROPIC_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let mut profile = get_provider_profile("anthropic").unwrap();
    let (server_url, requests, thread) = spawn_server(
        r#"{"data":[{"id":"claude-model-a"},{"name":"missing-id"},{"id":7},{"id":"claude-model-b"}]}"#,
    );

    // Test seam: the upstream method hard-codes the Anthropic endpoint. A
    // cloned Rust profile's explicit models_url lets the mock server observe
    // the same request construction without contacting the live API.
    profile.models_url = server_url;
    let models = profile.fetch_models(
        Some("sk-ant-test"),
        Some("http://127.0.0.1:1/ignored-by-anthropic"),
        8.0,
    );
    thread.join().unwrap();

    assert_eq!(
        models,
        Some(vec!["claude-model-a".into(), "claude-model-b".into()])
    );
    let request = requests.lock().unwrap().first().cloned().unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(lower.starts_with("get / "));
    assert!(lower.contains("x-api-key: sk-ant-test"));
    assert!(lower.contains("anthropic-version: 2023-06-01"));
    assert!(lower.contains("accept: application/json"));
    assert!(!lower.contains("authorization: bearer"));
}

#[test]
fn anthropic_model_fetch_fails_open_on_transport_or_json_errors() {
    let _guard = ANTHROPIC_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let mut profile = get_provider_profile("anthropic").unwrap();
    let (server_url, _, thread) = spawn_server("not-json");
    profile.models_url = server_url;

    assert_eq!(profile.fetch_models(Some("sk-ant-test"), None, 8.0), None);
    thread.join().unwrap();

    profile.models_url = "http://127.0.0.1:1/never-listens".into();
    assert_eq!(profile.fetch_models(Some("sk-ant-test"), None, 0.1), None);
}
