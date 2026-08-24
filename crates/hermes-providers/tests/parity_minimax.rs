//! Source-derived parity oracle for
//! `plugins/model-providers/minimax/__init__.py` @ b9aa928.
//!
//! The upstream profile tests pin the three auxiliary-model defaults; the
//! source hook adds MiniMax-M3 reasoning controls only for the global
//! OpenAI-compatible route. Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static MINIMAX_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(model: Option<&str>, base_url: Option<&str>) -> Map<String, Value> {
    let mut context = Map::new();
    if let Some(model) = model {
        context.insert("model".into(), Value::String(model.into()));
    }
    if let Some(base_url) = base_url {
        context.insert("base_url".into(), Value::String(base_url.into()));
    }
    context
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("JSON object")
}

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

#[test]
fn minimax_profiles_fields_aliases_and_aux_models_match_upstream() {
    let _guard = MINIMAX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();

    let global = get_provider_profile("minimax").expect("MiniMax must be registered");
    assert_eq!(global.name, "minimax");
    assert_eq!(global.aliases, ["mini-max"]);
    assert_eq!(global.api_mode, "anthropic_messages");
    assert!(global.display_name.is_empty());
    assert!(global.description.is_empty());
    assert!(global.signup_url.is_empty());
    assert_eq!(global.env_vars, ["MINIMAX_API_KEY"]);
    assert_eq!(global.base_url, "https://api.minimax.io/anthropic");
    assert_eq!(global.auth_type, "api_key");
    assert!(global.fallback_models.is_empty());
    assert_eq!(global.default_aux_model, "MiniMax-M3");
    assert!(global.minimax_reasoning);

    let china = get_provider_profile("minimax-cn").expect("MiniMax China must be registered");
    assert_eq!(china.aliases, ["minimax-china", "minimax_cn"]);
    assert_eq!(china.env_vars, ["MINIMAX_CN_API_KEY"]);
    assert_eq!(china.base_url, "https://api.minimaxi.com/anthropic");
    assert_eq!(china.auth_type, "api_key");
    assert_eq!(china.default_aux_model, "MiniMax-M3");
    assert!(china.minimax_reasoning);

    let oauth = get_provider_profile("minimax-oauth").expect("MiniMax OAuth must be registered");
    assert_eq!(oauth.aliases, ["minimax_oauth", "minimax-oauth-io"]);
    assert_eq!(oauth.api_mode, "anthropic_messages");
    assert_eq!(oauth.display_name, "MiniMax (OAuth)");
    assert_eq!(
        oauth.description,
        "MiniMax via OAuth browser flow — no API key required"
    );
    assert_eq!(oauth.signup_url, "https://api.minimax.io/");
    assert!(oauth.env_vars.is_empty());
    assert_eq!(oauth.base_url, "https://api.minimax.io/anthropic");
    assert_eq!(oauth.auth_type, "oauth_external");
    assert_eq!(oauth.default_aux_model, "MiniMax-M2.7");
    assert!(oauth.minimax_reasoning);

    for alias in [
        "mini-max",
        "minimax-china",
        "minimax_cn",
        "minimax_oauth",
        "minimax-oauth-io",
    ] {
        assert!(
            get_provider_profile(alias).is_some(),
            "alias {alias} must resolve"
        );
    }
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    for name in ["minimax", "minimax-cn", "minimax-oauth"] {
        assert_eq!(
            names.iter().filter(|candidate| *candidate == name).count(),
            1
        );
    }
}

#[test]
fn minimax_m3_global_openai_reasoning_shape_matches_upstream() {
    let _guard = MINIMAX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("minimax").unwrap();

    let (default_body, default_top) = profile.build_api_kwargs_extras(
        None,
        &context(Some("MiniMax-M3"), Some("https://api.minimax.io/v1")),
    );
    assert_eq!(default_body, object(json!({"reasoning_split": true})));
    assert!(default_top.is_empty());

    let adaptive = reasoning(Some(true), Some("high"));
    let (adaptive_body, adaptive_top) = profile.build_api_kwargs_extras(
        Some(&adaptive),
        &context(
            Some("minimax/minimax-m3"),
            Some("HTTPS://API.MINIMAX.IO/v1/"),
        ),
    );
    assert_eq!(
        adaptive_body,
        object(json!({
            "reasoning_split": true,
            "thinking": {"type": "adaptive"}
        }))
    );
    assert!(adaptive_top.is_empty());

    let empty_config = Map::new();
    let (empty_body, empty_top) = profile.build_api_kwargs_extras(
        Some(&empty_config),
        &context(Some("MiniMax-M3"), Some("https://api.minimax.io/v1")),
    );
    assert_eq!(
        empty_body,
        object(json!({
            "reasoning_split": true,
            "thinking": {"type": "adaptive"}
        }))
    );
    assert!(empty_top.is_empty());

    let disabled = reasoning(Some(false), Some("high"));
    let (disabled_body, disabled_top) = profile.build_api_kwargs_extras(
        Some(&disabled),
        &context(Some("MiniMax-M3"), Some("https://api.minimax.io/v1")),
    );
    assert_eq!(
        disabled_body,
        object(json!({
            "reasoning_split": true,
            "thinking": {"type": "disabled"}
        }))
    );
    assert!(disabled_top.is_empty());
}

#[test]
fn minimax_reasoning_hook_gates_model_and_global_openai_route() {
    let _guard = MINIMAX_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("minimax").unwrap();
    let config = reasoning(Some(true), Some("high"));

    for (model, base_url) in [
        ("MiniMax-M2.7", "https://api.minimax.io/v1"),
        ("MiniMax-M3", "https://api.minimax.io/anthropic"),
        ("MiniMax-M3", "https://api.minimaxi.com/v1"),
        ("custom-model", "https://api.minimax.io/v1"),
    ] {
        let (body, top) =
            profile.build_api_kwargs_extras(Some(&config), &context(Some(model), Some(base_url)));
        assert!(body.is_empty(), "unexpected body for {model} at {base_url}");
        assert!(
            top.is_empty(),
            "unexpected top-level fields for {model} at {base_url}"
        );
    }

    let (query_body, query_top) = profile.build_api_kwargs_extras(
        Some(&config),
        &context(
            Some("MiniMax-M3"),
            Some("https://api.minimax.io/v1?route=chat"),
        ),
    );
    assert_eq!(
        query_body,
        object(json!({
            "reasoning_split": true,
            "thinking": {"type": "adaptive"}
        }))
    );
    assert!(query_top.is_empty());

    let (no_model_body, no_model_top) = profile.build_api_kwargs_extras(
        Some(&config),
        &context(None, Some("https://api.minimax.io/v1")),
    );
    assert!(no_model_body.is_empty());
    assert!(no_model_top.is_empty());
}
