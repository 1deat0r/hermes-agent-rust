//! Source-derived parity oracle for
//! `plugins/model-providers/stepfun/__init__.py` @ b9aa928.
//!
//! The upstream module has no dedicated plugin-profile test file; its
//! declarative fields and registration side effect are the code oracle.
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static STEPFUN_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn stepfun_profile_fields_and_aliases_match_upstream() {
    let _guard = STEPFUN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("stepfun").expect("StepFun profile must be registered");

    assert_eq!(profile.name, "stepfun");
    assert_eq!(profile.aliases, ["step", "stepfun-coding-plan"]);
    assert_eq!(profile.env_vars, ["STEPFUN_API_KEY"]);
    assert_eq!(profile.base_url, "https://api.stepfun.ai/step_plan/v1");
    assert_eq!(profile.get_hostname(), "api.stepfun.ai");
    assert_eq!(profile.default_aux_model, "step-3.5-flash");
    assert_eq!(get_provider_profile("step").unwrap().name, "stepfun");
    assert_eq!(
        get_provider_profile("stepfun-coding-plan").unwrap().name,
        "stepfun"
    );
}

#[test]
fn stepfun_is_listed_once_by_canonical_name() {
    let _guard = STEPFUN_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "stepfun").count(), 1);
}
