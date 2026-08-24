//! Source-derived parity oracle for
//! \`plugins/model-providers/ollama-cloud/__init__.py\` @ b9aa928.
//!
//! The upstream profile test pins the top-level \`reasoning_effort\` wire shape;
//! the Rust fixture mirrors its metadata, capability gate, normalization,
//! disable switch, and fail-open effort handling. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{Map, Value};

static OLLAMA_CLOUD_TEST_LOCK: Mutex<()> = Mutex::new(());

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

fn supports_reasoning(value: bool) -> Map<String, Value> {
    Map::from_iter([("supports_reasoning".into(), Value::Bool(value))])
}

#[test]
fn ollama_cloud_profile_fields_aliases_and_aux_model_match_upstream() {
    let _guard = OLLAMA_CLOUD_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("ollama-cloud").expect("Ollama Cloud must be registered");

    assert_eq!(profile.name, "ollama-cloud");
    assert_eq!(profile.aliases, ["ollama_cloud"]);
    assert_eq!(profile.env_vars, ["OLLAMA_API_KEY"]);
    assert_eq!(profile.base_url, "https://ollama.com/v1");
    assert_eq!(profile.auth_type, "api_key");
    assert_eq!(profile.api_mode, "chat_completions");
    assert_eq!(profile.default_aux_model, "nemotron-3-nano:30b");
    assert!(profile.fallback_models.is_empty());
    assert!(profile.ollama_cloud_reasoning);
    assert_eq!(
        get_provider_profile("ollama_cloud").unwrap().name,
        "ollama-cloud"
    );

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        names.iter().filter(|name| *name == "ollama-cloud").count(),
        1
    );
}

#[test]
fn ollama_cloud_reasoning_maps_supported_efforts_to_top_level() {
    let _guard = OLLAMA_CLOUD_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("ollama-cloud").unwrap();
    let context = supports_reasoning(true);

    for effort in ["xhigh", "max", "MAX", "  Max  ", "ultra"] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&config), &context);
        assert!(extra_body.is_empty());
        assert_eq!(
            top_level.get("reasoning_effort"),
            Some(&Value::String("max".into()))
        );
    }

    for effort in ["low", "medium", "high"] {
        let config = reasoning(Some(true), Some(effort));
        let (extra_body, top_level) = profile.build_api_kwargs_extras(Some(&config), &context);
        assert!(extra_body.is_empty());
        assert_eq!(
            top_level.get("reasoning_effort"),
            Some(&Value::String(effort.into()))
        );
    }
}

#[test]
fn ollama_cloud_reasoning_disable_gate_and_unknown_efforts_match_upstream() {
    let _guard = OLLAMA_CLOUD_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("ollama-cloud").unwrap();

    let disabled = reasoning(Some(false), Some("high"));
    let (disabled_body, disabled_top) =
        profile.build_api_kwargs_extras(Some(&disabled), &supports_reasoning(true));
    assert!(disabled_body.is_empty());
    assert_eq!(
        disabled_top.get("reasoning_effort"),
        Some(&Value::String("none".into()))
    );

    let explicit_none = reasoning(Some(true), Some("none"));
    let (_, none_top) =
        profile.build_api_kwargs_extras(Some(&explicit_none), &supports_reasoning(true));
    assert_eq!(
        none_top.get("reasoning_effort"),
        Some(&Value::String("none".into()))
    );

    for effort in ["", "future-tier", "minimal"] {
        let config = reasoning(Some(true), Some(effort));
        let (_, top_level) =
            profile.build_api_kwargs_extras(Some(&config), &supports_reasoning(true));
        assert!(top_level.is_empty(), "effort {effort:?} must be omitted");
    }

    let (_, no_config_top) = profile.build_api_kwargs_extras(None, &supports_reasoning(true));
    assert!(no_config_top.is_empty());

    let config = reasoning(Some(true), Some("xhigh"));
    let (unsupported_body, unsupported_top) =
        profile.build_api_kwargs_extras(Some(&config), &supports_reasoning(false));
    assert!(unsupported_body.is_empty());
    assert!(unsupported_top.is_empty());
}
