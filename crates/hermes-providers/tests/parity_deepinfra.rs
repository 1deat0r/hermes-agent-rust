//! Source-derived parity oracle for
//! `plugins/model-providers/deepinfra/__init__.py` @ b9aa928.
//!
//! The upstream profile behavior is covered by
//! `tests/hermes_cli/test_api_key_providers.py`; the catalog fixture mirrors
//! its DeepInfra tag-filtering and profile-hook cases. Tier: unit/mock.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static DEEPINFRA_TEST_LOCK: Mutex<()> = Mutex::new(());

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
fn deepinfra_profile_fields_aliases_and_auxiliary_model_match_upstream() {
    let _guard = DEEPINFRA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("deepinfra").expect("DeepInfra must be registered");

    assert_eq!(profile.name, "deepinfra");
    assert_eq!(profile.aliases, ["deep-infra", "deepinfra-ai"]);
    assert_eq!(profile.display_name, "DeepInfra");
    assert_eq!(
        profile.description,
        "DeepInfra — 100+ open models, pay-per-use"
    );
    assert_eq!(profile.signup_url, "https://deepinfra.com/dash/api_keys");
    assert_eq!(
        profile.env_vars,
        ["DEEPINFRA_API_KEY", "DEEPINFRA_BASE_URL"]
    );
    assert_eq!(profile.base_url, "https://api.deepinfra.com/v1/openai");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.default_max_tokens, None);
    assert_eq!(profile.default_aux_model, "deepseek-ai/DeepSeek-V4-Flash");
    assert!(profile.fallback_models.is_empty());
    assert!(profile.deepinfra_vision);
    assert_eq!(profile.get_hostname(), "api.deepinfra.com");
    assert_eq!(
        get_provider_profile("deep-infra").unwrap().name,
        "deepinfra"
    );
    assert_eq!(
        get_provider_profile("deepinfra-ai").unwrap().name,
        "deepinfra"
    );

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "deepinfra").count(), 1);
}

#[test]
fn deepinfra_vision_hook_is_key_gated_and_selects_chat_vision_model() {
    let _guard = DEEPINFRA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("deepinfra").unwrap();

    std::env::remove_var("DEEPINFRA_API_KEY");
    assert_eq!(profile.default_vision_model(), None);

    let (base_url, server) = spawn_catalog_server(
        r#"{"data":[
            {"id":"vendor/image-vision","metadata":{"tags":["image-gen","vision"]}},
            {"id":"vendor/chat-plain","metadata":{"tags":["chat"]}},
            {"id":"vendor/chat-vision","metadata":{"tags":["chat","vision"]}},
            {"id":"stub-model","metadata":null}
        ]}"#,
    );
    std::env::set_var("DEEPINFRA_API_KEY", "test-deepinfra-key");
    std::env::set_var("DEEPINFRA_BASE_URL", &base_url);

    assert_eq!(
        profile.default_vision_model(),
        Some("vendor/chat-vision".into())
    );
    // The source helper caches the raw catalog by base URL for the process;
    // the second hook call must reuse the one loopback response.
    assert_eq!(
        profile.default_vision_model(),
        Some("vendor/chat-vision".into())
    );
    let request = server.join().unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(lower.starts_with("get /models?filter=true&sort_by=hermes http/1.1"));
    assert!(lower.contains("authorization: bearer test-deepinfra-key"));

    std::env::remove_var("DEEPINFRA_API_KEY");
    std::env::remove_var("DEEPINFRA_BASE_URL");
}
