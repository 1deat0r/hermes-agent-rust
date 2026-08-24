//! Source-derived parity oracle for
//! `plugins/model-providers/actual/__init__.py` @ b9aa928.
//!
//! The dedicated upstream `tests/hermes_cli/test_actual_provider.py` covers
//! profile metadata and the custom catalog hook. The Rust fixture exercises
//! the same URL, header, response-shape, and fail-open behavior. Tier:
//! unit/mock.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static ACTUAL_TEST_LOCK: Mutex<()> = Mutex::new(());

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read catalog request");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn spawn_catalog_server(payload: &'static str) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind catalog server");
    let address = listener.local_addr().expect("catalog server address");
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept catalog request");
        let request = read_request(&mut stream);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            payload.len(),
            payload
        );
        stream
            .write_all(response.as_bytes())
            .expect("write catalog response");
        request
    });
    (format!("http://{address}"), thread)
}

#[test]
fn actual_profile_fields_aliases_and_codex_mode_match_upstream() {
    let _guard = ACTUAL_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("actual-computer").expect("Actual must be registered");

    assert_eq!(profile.name, "actual");
    assert_eq!(
        profile.aliases,
        ["actual-computer", "actualcomputer", "aci"]
    );
    assert_eq!(profile.display_name, "Actual Computer");
    assert_eq!(
        profile.description,
        "Actual Computer - hosted inference via api.actual.inc, or local offline inference via ACTUAL_BASE_URL"
    );
    assert_eq!(profile.signup_url, "https://actual.inc");
    assert_eq!(profile.env_vars, ["ACTUAL_API_KEY", "ACTUAL_BASE_URL"]);
    assert_eq!(profile.base_url, "https://api.actual.inc/v1");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.api_mode, "codex_responses");
    assert!(profile.fallback_models.is_empty());
    assert!(profile.default_aux_model.is_empty());
    assert!(profile.actual_catalog);
    assert_eq!(
        get_provider_profile("actualcomputer").unwrap().name,
        "actual"
    );
    assert_eq!(get_provider_profile("aci").unwrap().name, "actual");

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "actual").count(), 1);
}

#[test]
fn actual_catalog_normalizes_env_base_url_and_accepts_object_data() {
    let _guard = ACTUAL_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("actual").unwrap();
    let (base_url, server) =
        spawn_catalog_server(r#"{"data":[{"id":"actual/local-model"},{"name":"ignored"}]}"#);

    std::env::remove_var("ACTUAL_API_KEY");
    std::env::set_var("ACTUAL_BASE_URL", &base_url);
    let models = profile.fetch_models(None, Some("http://127.0.0.1:1"), 1.5);
    assert_eq!(models, Some(vec!["actual/local-model".into()]));

    let request = server.join().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("get /v1/models http/1.1"));
    assert!(request.contains("accept: application/json"));
    assert!(request.contains("user-agent: hermes-cli"));
    assert!(!request.contains("authorization:"));
    std::env::remove_var("ACTUAL_BASE_URL");
}

#[test]
fn actual_catalog_sends_optional_bearer_and_fails_open_on_bad_json() {
    let _guard = ACTUAL_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("actual").unwrap();

    let (base_url, server) = spawn_catalog_server(r#"[{"id":"actual/hosted-model"}]"#);
    let explicit_base_url = format!("{base_url}/v1");
    let models = profile.fetch_models(Some("actual-test-key"), Some(&explicit_base_url), 1.5);
    assert_eq!(models, Some(vec!["actual/hosted-model".into()]));
    let request = server.join().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("get /v1/models http/1.1"));
    assert!(request.contains("authorization: bearer actual-test-key"));

    let (bad_base_url, bad_server) = spawn_catalog_server("not-json");
    assert_eq!(profile.fetch_models(None, Some(&bad_base_url), 1.5), None);
    let _ = bad_server.join().unwrap();
}
