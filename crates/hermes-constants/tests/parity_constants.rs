//! Parity oracles for `hermes-constants` vs upstream `hermes_constants.py`
//! (@ b9aa928). Golden fixtures live in `upstream/` (read-only).
//!
//! Covers the public-surface functions whose behavior is pure or env-driven
//! without invasive file probing: reasoning-effort parsing, model-variant
//! generation, venv path layout, module identification, WSL path translation.

use hermes_constants::{
    canonical_model_variants, is_first_party_module, parse_reasoning_effort,
    venv_bin_dir, venv_python_path, windows_path_to_wsl, wsl_unc_path_to_posix,
    Platform, FIRST_PARTY_MODULE_ROOTS,
};
use serde_json::Value;
use std::sync::Mutex;

// Tests that touch env vars are serialized to avoid cross-test races.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn load_golden() -> Value {
    // Path relative to workspace root; integration tests run from the crate dir.
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path.push("upstream/golden_constants_reasoning.json");
    let text = std::fs::read_to_string(&path).expect("golden fixture missing — run tools/gen_golden.sh");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn parse_reasoning_effort_matches_upstream_golden() {
    let golden = load_golden();
    let pc = &golden["parse_reasoning_effort"];

    let expect = |key: &str, input: Option<hermes_constants::ReasoningConfig>| {
        let g = &pc[key];
        match input {
            None => assert!(g.is_null(), "key {}: expected None, got {:?}", key, g),
            Some(cfg) => {
                assert_eq!(g["enabled"].as_bool().unwrap(), cfg.enabled, "key {}", key);
                match &cfg.effort {
                    Some(e) => assert_eq!(g["effort"].as_str().unwrap(), e, "key {}", key),
                    None => assert!(g.get("effort").is_none(), "key {}", key),
                }
            }
        }
    };

    expect(r#"False"#, parse_reasoning_effort(false));
    expect(r#"True"#, parse_reasoning_effort(true));
    expect(r#"None"#, parse_reasoning_effort(None::<&str>));
    expect(r#"''"#, parse_reasoning_effort(""));
    expect(r#"'  '"#, parse_reasoning_effort("  "));
    expect(r#"'none'"#, parse_reasoning_effort("none"));
    expect(r#"'false'"#, parse_reasoning_effort("false"));
    expect(r#"'disabled'"#, parse_reasoning_effort("disabled"));
    expect(r#"'minimal'"#, parse_reasoning_effort("minimal"));
    expect(r#"'MEDIUM'"#, parse_reasoning_effort("MEDIUM"));
    expect(r#"'  High '"#, parse_reasoning_effort("  High "));
    expect(r#"'bogus'"#, parse_reasoning_effort("bogus"));
    expect(r#"'xhigh'"#, parse_reasoning_effort("xhigh"));
    expect(r#"'ultra'"#, parse_reasoning_effort("ultra"));
}

/// Exact full-list equality against the golden upstream output.
#[test]
fn canonical_model_variants_matches_upstream_golden_exactly() {
    let golden = load_golden();
    let cases = &golden["canonical_model_variants"]["cases"];
    let cases = cases.as_object().unwrap();
    for (model, expected) in cases {
        let expected: Vec<String> = expected
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let got = canonical_model_variants(model);
        assert_eq!(
            got.len(),
            expected.len(),
            "variant count mismatch for {:?}: got {} expected {}",
            model,
            got.len(),
            expected.len()
        );
        for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            assert_eq!(g, e, "variant[{}] mismatch for {:?}", i, model);
        }
        assert_eq!(&got[..], &expected[..], "exact list mismatch for {:?}", model);
    }
}

#[test]
fn canonical_model_variants_spot_checks() {
    let v = canonical_model_variants("claude-opus-4.5");
    assert_eq!(v[0], "claude-opus-4.5"); // exact always first
    assert_eq!(v[1], "claude-opus-4-5"); // dot→dash derivative second
    assert!(v.contains(&"openrouter/anthropic/claude-opus-4.5".to_string()));
}

#[test]
fn venv_layout_matches_upstream() {
    use std::path::PathBuf;
    // upstream: venv_bin_dir("/venv", windows=False) -> /venv/bin
    assert_eq!(venv_bin_dir("/venv", Some(Platform::Posix)), PathBuf::from("/venv/bin"));
    assert_eq!(venv_bin_dir("/venv", Some(Platform::Windows)), PathBuf::from("/venv/Scripts"));
    assert_eq!(
        venv_python_path("/venv", Some(Platform::Posix)),
        PathBuf::from("/venv/bin/python")
    );
    assert_eq!(
        venv_python_path("/venv", Some(Platform::Windows)),
        PathBuf::from("/venv/Scripts/python.exe")
    );
}

#[test]
fn first_party_module_roots_exact() {
    // upstream frozenset order is irrelevant; membership is the contract.
    for root in FIRST_PARTY_MODULE_ROOTS {
        assert!(is_first_party_module(Some(root)), "{} should be first-party", root);
        assert!(is_first_party_module(Some(&format!("{}.thing", root))));
    }
    for bad in ["agents", "agentops", "toolsets_x", "hermesx", "", "unknown"] {
        assert!(!is_first_party_module(Some(bad)), "{} should NOT be first-party", bad);
    }
    assert!(!is_first_party_module(None));
}

#[test]
fn wsl_path_translation_matches_upstream() {
    assert_eq!(
        windows_path_to_wsl("C:\\Users\\me"),
        Some("/mnt/c/Users/me".to_string())
    );
    assert_eq!(windows_path_to_wsl("/linux/path"), None);
    assert_eq!(
        wsl_unc_path_to_posix("\\\\wsl.localhost\\Ubuntu\\home\\me"),
        Some("/home/me".to_string())
    );
    assert_eq!(
        wsl_unc_path_to_posix("\\\\wsl$\\Ubuntu\\"),
        Some("/".to_string())
    );
}

#[test]
fn env_driven_dirs() {
    let _g = ENV_MUTEX.lock().unwrap();
    let td = tempfile::TempDir::new().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", td.path()) };
    unsafe { std::env::set_var("HERMES_OPTIONAL_SKILLS", "/pkg/opt") };
    unsafe { std::env::remove_var("HERMES_OPTIONAL_MCPS") };

    assert_eq!(
        hermes_constants::get_config_path(),
        td.path().join("config.yaml")
    );
    assert_eq!(
        hermes_constants::get_optional_skills_dir(None),
        std::path::PathBuf::from("/pkg/opt")
    );
    assert_eq!(
        hermes_constants::get_optional_mcps_dir(None),
        td.path().join("optional-mcps")
    );
    assert_eq!(hermes_constants::get_env_path(), td.path().join(".env"));
    assert_eq!(hermes_constants::get_skills_dir(), td.path().join("skills"));

    unsafe { std::env::remove_var("HERMES_HOME") };
    unsafe { std::env::remove_var("HERMES_OPTIONAL_SKILLS") };
}

#[test]
fn home_override_is_process_scoped() {
    let _g = ENV_MUTEX.lock().unwrap();
    unsafe { std::env::set_var("HERMES_HOME", "/env/home") };
    assert_eq!(
        hermes_constants::get_process_hermes_home(),
        std::path::PathBuf::from("/env/home")
    );
    let tok = hermes_constants::set_hermes_home_override(Some("/ctx/home"));
    assert_eq!(
        hermes_constants::get_hermes_home(),
        std::path::PathBuf::from("/ctx/home")
    );
    // process home ignores the override (upstream contract)
    assert_eq!(
        hermes_constants::get_process_hermes_home(),
        std::path::PathBuf::from("/env/home")
    );
    hermes_constants::reset_hermes_home_override(tok);
    assert_eq!(
        hermes_constants::get_hermes_home(),
        std::path::PathBuf::from("/env/home")
    );
    unsafe { std::env::remove_var("HERMES_HOME") };
}
