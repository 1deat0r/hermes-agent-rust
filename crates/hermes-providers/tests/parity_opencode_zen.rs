//! PARITY: `plugins/model-providers/opencode-zen/__init__.py` lines 18-147 and
//! `tests/plugins/model_providers/test_opencode_go_profile.py` lines 19-157.

use hermes_providers::{get_provider_profile, ProviderProfile};
use serde_json::{json, Map, Value};

fn profile(name: &str) -> ProviderProfile {
    get_provider_profile(name).unwrap_or_else(|| panic!("missing provider profile {name}"))
}

fn context(model: Option<&str>) -> Map<String, Value> {
    model
        .map(|model| Map::from_iter([(String::from("model"), Value::String(model.into()))]))
        .unwrap_or_default()
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
fn metadata_and_aliases_match_upstream_profiles() {
    let zen = profile("opencode-zen");
    assert_eq!(zen.aliases, vec!["opencode", "opencode_zen", "zen"]);
    assert_eq!(zen.env_vars, vec!["OPENCODE_ZEN_API_KEY"]);
    assert_eq!(zen.base_url, "https://opencode.ai/zen/v1");
    assert_eq!(zen.default_aux_model, "gemini-3-flash");

    let go = profile("opencode-go");
    assert_eq!(go.aliases, vec!["opencode_go", "go", "opencode-go-sub"]);
    assert_eq!(go.env_vars, vec!["OPENCODE_GO_API_KEY"]);
    assert_eq!(go.base_url, "https://opencode.ai/zen/go/v1");
    assert_eq!(go.default_aux_model, "glm-5");
    assert!(go.opencode_go_reasoning);

    for alias in ["opencode", "opencode_zen", "zen"] {
        assert_eq!(profile(alias).name, "opencode-zen");
    }
    for alias in ["opencode_go", "go", "opencode-go-sub"] {
        assert_eq!(profile(alias).name, "opencode-go");
    }
}

#[test]
fn kimi_k2_normalizes_aggregator_model_and_reasoning_wire_shape() {
    let go = profile("opencode-go");

    let (body, top) = go.build_api_kwargs_extras(
        Some(&reasoning(Some(false), None)),
        &context(Some("  MoonShotAI/KIMI-K2.6  ")),
    );
    assert_eq!(
        body,
        Map::from_iter([(String::from("thinking"), json!({"type": "disabled"}))])
    );
    assert!(top.is_empty());

    for (effort, expected) in [("low", "low"), ("medium", "medium"), ("high", "high")] {
        let (body, top) = go.build_api_kwargs_extras(
            Some(&reasoning(Some(true), Some(effort))),
            &context(Some("kimi-k2.5")),
        );
        assert!(
            body.is_empty(),
            "effort {effort} unexpectedly emitted thinking"
        );
        assert_eq!(top.get("reasoning_effort"), Some(&json!(expected)));
    }
    for effort in ["xhigh", "max", "ultra"] {
        let (body, top) = go.build_api_kwargs_extras(
            Some(&reasoning(Some(true), Some(effort))),
            &context(Some("kimi-k2.6")),
        );
        assert!(body.is_empty());
        assert_eq!(top.get("reasoning_effort"), Some(&json!("high")));
    }

    let (body, top) = go.build_api_kwargs_extras(
        Some(&reasoning(Some(true), Some("minimal"))),
        &context(Some("kimi-k2.6")),
    );
    assert_eq!(body.get("thinking"), Some(&json!({"type": "enabled"})));
    assert!(top.is_empty());

    let (body, top) = go.build_api_kwargs_extras(None, &context(Some("kimi-k2.6")));
    assert!(body.is_empty());
    assert!(top.is_empty());
}

#[test]
fn deepseek_v4_and_reasoner_normalize_reasoning_without_conflicting_shapes() {
    let go = profile("opencode-go");
    for model in ["deepseek/deepseek-v4-pro", " DEEPSEEK-REASONER "] {
        let (body, top) = go.build_api_kwargs_extras(
            Some(&reasoning(Some(false), Some("high"))),
            &context(Some(model)),
        );
        assert_eq!(body.get("thinking"), Some(&json!({"type": "disabled"})));
        assert!(top.is_empty());

        for (effort, expected) in [
            ("low", "low"),
            ("medium", "medium"),
            ("high", "high"),
            ("xhigh", "max"),
            ("max", "max"),
            ("ultra", "max"),
        ] {
            let (body, top) = go.build_api_kwargs_extras(
                Some(&reasoning(Some(true), Some(effort))),
                &context(Some(model)),
            );
            assert!(
                body.is_empty(),
                "effort {effort} emitted conflicting thinking"
            );
            assert_eq!(top.get("reasoning_effort"), Some(&json!(expected)));
        }

        let (body, top) = go.build_api_kwargs_extras(
            Some(&reasoning(Some(true), Some("unknown"))),
            &context(Some(model)),
        );
        assert_eq!(body.get("thinking"), Some(&json!({"type": "enabled"})));
        assert!(top.is_empty());

        let (body, top) = go.build_api_kwargs_extras(None, &context(Some(model)));
        assert_eq!(body.get("thinking"), Some(&json!({"type": "enabled"})));
        assert!(top.is_empty());
    }

    for model in ["deepseek-v3.1", "deepseek-chat"] {
        let (body, top) = go.build_api_kwargs_extras(
            Some(&reasoning(Some(true), Some("high"))),
            &context(Some(model)),
        );
        assert!(body.is_empty(), "unexpected body for {model}");
        assert!(top.is_empty(), "unexpected top-level kwargs for {model}");
    }
}

#[test]
fn glm_52_aliases_map_only_supported_effort_levels() {
    let go = profile("opencode-go");
    for model in ["glm-5.2", "glm-5-2", "glm-5p2", "provider/GLM-5P2"] {
        for effort in ["low", "medium", "high"] {
            let (body, top) = go.build_api_kwargs_extras(
                Some(&reasoning(Some(true), Some(effort))),
                &context(Some(model)),
            );
            assert!(body.is_empty());
            assert_eq!(top.get("reasoning_effort"), Some(&json!("high")));
        }
        for effort in ["xhigh", "max", "ultra"] {
            let (body, top) = go.build_api_kwargs_extras(
                Some(&reasoning(Some(true), Some(effort))),
                &context(Some(model)),
            );
            assert!(body.is_empty());
            assert_eq!(top.get("reasoning_effort"), Some(&json!("max")));
        }
        for config in [
            None,
            Some(reasoning(Some(false), None)),
            Some(reasoning(Some(true), Some("none"))),
            Some(Map::new()),
        ] {
            let (body, top) = go.build_api_kwargs_extras(config.as_ref(), &context(Some(model)));
            assert!(body.is_empty());
            assert!(top.is_empty());
        }
    }
}

#[test]
fn max_tokens_caps_only_mimo_v25_pro_after_normalization() {
    let go = profile("opencode-go");
    assert_eq!(go.get_max_tokens(Some(" mimo-v2.5-pro ")), Some(131_072));
    assert_eq!(
        go.get_max_tokens(Some("xiaomi/mimo-v2.5-pro")),
        Some(131_072)
    );
    assert_eq!(go.get_max_tokens(Some("mimo-v2.5")), None);
    assert_eq!(go.get_max_tokens(Some("glm-5.2")), None);
    assert_eq!(go.get_max_tokens(None), None);
}
