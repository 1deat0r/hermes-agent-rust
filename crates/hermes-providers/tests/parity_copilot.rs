//! Parity oracle for
//! `plugins/model-providers/copilot/__init__.py` @ b9aa928 and
//! `tests/plugins/model_providers/test_copilot_profile.py`.
//!
//! Tier: unit. The upstream live model-catalog helper is represented by the
//! explicit `supported_efforts` context seam until the CLI/model crate exists.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};
use serde_json::{json, Map, Value};

static COPILOT_TEST_LOCK: Mutex<()> = Mutex::new(());

fn context(model: Option<&str>, supports_reasoning: bool, efforts: Value) -> Map<String, Value> {
    let mut context = Map::new();
    if let Some(model) = model {
        context.insert("model".into(), Value::String(model.into()));
    }
    context.insert("supports_reasoning".into(), Value::Bool(supports_reasoning));
    context.insert("supported_efforts".into(), efforts);
    context
}

fn reasoning(effort: &str) -> Map<String, Value> {
    Map::from_iter([("effort".into(), Value::String(effort.into()))])
}

fn reasoning_effort(body: &Map<String, Value>) -> Option<&str> {
    body.get("reasoning")
        .and_then(Value::as_object)
        .and_then(|reasoning| reasoning.get("effort"))
        .and_then(Value::as_str)
}

#[test]
fn copilot_profile_fields_aliases_and_registration_match_upstream() {
    let _guard = COPILOT_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("copilot").expect("Copilot profile must be registered");

    assert_eq!(profile.name, "copilot");
    assert_eq!(
        profile.aliases,
        ["github-copilot", "github-models", "github-model", "github"]
    );
    assert!(profile.display_name.is_empty());
    assert!(profile.description.is_empty());
    assert!(profile.signup_url.is_empty());
    assert!(profile.fallback_models.is_empty());
    assert!(profile.default_aux_model.is_empty());
    assert_eq!(
        profile.env_vars,
        ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]
    );
    assert_eq!(profile.base_url, "https://api.githubcopilot.com");
    assert_eq!(profile.auth_type, "copilot");
    assert!(profile.copilot_reasoning);
    for alias in ["github-copilot", "github-models", "github-model", "github"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "copilot");
    }

    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "copilot").count(), 1);
}

#[test]
fn copilot_reasoning_effort_clamp_precedence_matches_upstream() {
    let _guard = COPILOT_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("copilot").unwrap();
    let all_efforts = json!(["minimal", "low", "medium", "high", "xhigh"]);

    let (_, top_level) = profile.build_api_kwargs_extras(
        Some(&reasoning("xhigh")),
        &context(Some("gpt-5.5"), true, all_efforts.clone()),
    );
    assert!(top_level.is_empty());
    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("xhigh")),
        &context(Some("gpt-5.5"), true, all_efforts),
    );
    assert_eq!(reasoning_effort(&body.0), Some("xhigh"));

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("xhigh")),
        &context(
            Some("o-series-model"),
            true,
            json!(["low", "medium", "high"]),
        ),
    );
    assert_eq!(reasoning_effort(&body.0), Some("high"));

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("minimal")),
        &context(
            Some("o-series-model"),
            true,
            json!(["low", "medium", "high"]),
        ),
    );
    assert_eq!(reasoning_effort(&body.0), Some("low"));

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("garbage")),
        &context(Some("some-model"), true, json!(["low", "medium", "high"])),
    );
    assert_eq!(reasoning_effort(&body.0), Some("medium"));

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("garbage")),
        &context(Some("weird-model"), true, json!(["low", "high"])),
    );
    assert_eq!(reasoning_effort(&body.0), Some("low"));
}

#[test]
fn copilot_reasoning_is_fail_open_when_catalog_or_capability_is_missing() {
    let _guard = COPILOT_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("copilot").unwrap();

    let body =
        profile.build_api_kwargs_extras(None, &context(Some("gpt-5.5"), true, json!(["low"])));
    assert_eq!(reasoning_effort(&body.0), Some("medium"));

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("high")),
        &context(Some("gpt-5.5"), false, json!(["low", "medium", "high"])),
    );
    assert!(body.0.is_empty());
    assert!(body.1.is_empty());

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("high")),
        &context(None, true, json!(["low", "medium", "high"])),
    );
    assert!(body.0.is_empty());

    let body = profile.build_api_kwargs_extras(
        Some(&reasoning("high")),
        &context(Some("gpt-5.5"), true, Value::Null),
    );
    assert!(body.0.is_empty());
}
