//! Parity oracles for the Kimi Coding provider module at upstream commit
//! b9aa928.
//!
//! Tier: mock/unit. The model-catalog cases use a loopback HTTP server for the
//! same fail-open/filtering behavior exercised by the upstream monkeypatch.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};

use hermes_providers::registry::get_provider_profile;
use hermes_providers::{FixedTemperature, OMIT_TEMPERATURE};
use serde_json::{Map, Value};

fn reasoning(enabled: Option<bool>, effort: Option<&str>) -> Map<String, Value> {
    let mut config = Map::new();
    if let Some(enabled) = enabled {
        config.insert("enabled".into(), Value::Bool(enabled));
    }
    if let Some(effort) = effort {
        config.insert("effort".into(), Value::String(effort.into()));
    }
    config
}

fn spawn_model_server() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let address = listener.local_addr().expect("server address");
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept model request");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).expect("read model request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let body = r#"{"data":[{"id":"k3"},{"id":" K3 "},{"id":"kimi-k2.6"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write model response");
    });
    (format!("http://{address}"), thread)
}

#[test]
fn kimi_profiles_match_source_fields_and_aliases() {
    let global = get_provider_profile("kimi-coding").expect("Kimi Coding must be registered");
    assert_eq!(global.name, "kimi-coding");
    assert_eq!(global.aliases, ["kimi", "moonshot", "kimi-for-coding"]);
    assert_eq!(global.env_vars, ["KIMI_API_KEY", "KIMI_CODING_API_KEY"]);
    assert_eq!(global.base_url, "https://api.moonshot.ai/v1");
    assert_eq!(global.fixed_temperature, OMIT_TEMPERATURE);
    assert_eq!(global.fixed_temperature, FixedTemperature::Omit);
    assert_eq!(global.default_max_tokens, Some(32_000));
    assert_eq!(
        global.default_headers.get("User-Agent"),
        Some(&"hermes-agent/1.0".to_owned())
    );
    assert_eq!(global.default_aux_model, "kimi-k2-turbo-preview");
    assert!(global.kimi_coding);

    let china = get_provider_profile("kimi-coding-cn").expect("Kimi China must be registered");
    assert_eq!(china.name, "kimi-coding-cn");
    assert_eq!(china.aliases, ["kimi-cn", "moonshot-cn"]);
    assert_eq!(china.env_vars, ["KIMI_CN_API_KEY"]);
    assert_eq!(china.base_url, "https://api.moonshot.cn/v1");
    assert_eq!(china.fixed_temperature, OMIT_TEMPERATURE);
    assert_eq!(china.default_max_tokens, Some(32_000));
    assert_eq!(china.default_aux_model, "kimi-k2-turbo-preview");
    assert!(china.kimi_coding);

    assert_eq!(get_provider_profile("kimi").unwrap().name, "kimi-coding");
    assert_eq!(
        get_provider_profile("moonshot").unwrap().name,
        "kimi-coding"
    );
    assert_eq!(
        get_provider_profile("kimi-cn").unwrap().name,
        "kimi-coding-cn"
    );
    assert_eq!(
        get_provider_profile("moonshot-cn").unwrap().name,
        "kimi-coding-cn"
    );
}

#[test]
fn kimi_reasoning_wire_shape_is_mutually_exclusive() {
    let profile = get_provider_profile("kimi-coding").unwrap();

    let (extra_body, top_level) = profile.build_api_kwargs_extras(None, &Map::new());
    assert_eq!(
        extra_body.get("thinking"),
        Some(&serde_json::json!({"type": "enabled"}))
    );
    assert!(top_level.is_empty());

    let enabled_without_effort = reasoning(Some(true), None);
    let (extra_body, top_level) =
        profile.build_api_kwargs_extras(Some(&enabled_without_effort), &Map::new());
    assert_eq!(
        extra_body.get("thinking"),
        Some(&serde_json::json!({"type": "enabled"}))
    );
    assert!(top_level.is_empty());

    for effort in ["low", "medium", "high"] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&config), &Map::new());
        assert!(extra_body.get("thinking").is_none());
        assert_eq!(
            top_level.get("reasoning_effort"),
            Some(&Value::String(effort.into()))
        );
    }

    for effort in ["", "garbage", "xhigh", "max"] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&config), &Map::new());
        assert_eq!(
            extra_body.get("thinking"),
            Some(&serde_json::json!({"type": "enabled"}))
        );
        assert!(top_level.is_empty());
    }

    let disabled = reasoning(Some(false), None);
    let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&disabled), &Map::new());
    assert_eq!(
        extra_body.get("thinking"),
        Some(&serde_json::json!({"type": "disabled"}))
    );
    assert!(top_level.is_empty());

    for config in [
        None,
        Some(reasoning(Some(true), None)),
        Some(reasoning(Some(true), Some("high"))),
        Some(reasoning(Some(true), Some("garbage"))),
        Some(reasoning(Some(false), None)),
        Some(reasoning(Some(false), Some("low"))),
    ] {
        let (extra_body, top_level) = profile.build_api_kwargs_extras(config.as_ref(), &Map::new());
        assert!(
            !(extra_body.contains_key("thinking") && top_level.contains_key("reasoning_effort"))
        );
    }
}

#[test]
fn malformed_unconfirmed_url_filters_k3_using_standard_catalog_seam() {
    let mut profile = get_provider_profile("kimi-coding").unwrap().clone();
    let (server_url, thread) = spawn_model_server();
    // models_url is the Rust equivalent of the upstream test's patched
    // ProviderProfile.fetch_models: it supplies a deterministic catalog while
    // the malformed caller URL still exercises the unconfirmed branch.
    profile.models_url = format!("{server_url}/models");

    let models = profile.fetch_models(Some("test-key"), Some("https://[api.kimi.com/coding"), 8.0);
    thread.join().unwrap();
    assert_eq!(models, Some(vec!["kimi-k2.6".into()]));
}
