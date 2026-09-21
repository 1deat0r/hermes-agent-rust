//! Parity tests for `agent/verify/runner.py` @ 5d59366. Upstream has no
//! dedicated test file (missing-test gap, noted in the ledger); cases
//! derive from the upstream code as oracle. Only fast shell recipes and
//! loopback readiness probes are exercised (the same trust level the
//! module documents for the project's own commands).

use std::fs;

use serde_json::json;

use hermes_agent::verify::recipes::{detect_recipe, Recipe};
use hermes_agent::verify::runner::{run_verify, PHASE_ORDER};

fn make_project() -> tempfile::TempDir {
    let td = tempfile::TempDir::new().unwrap();
    fs::write(td.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    td
}

fn rust_recipe(start: Option<&str>) -> Recipe {
    Recipe {
        name: "Rust project".to_string(),
        kind: "rust".to_string(),
        build: vec!["echo building".to_string()],
        test: vec!["echo testing".to_string()],
        start: start.map(|s| s.to_string()),
        port: Some(8123),
        readiness_path: "/".to_string(),
        ..Recipe::from_dict(&json!({"name": "Rust project", "kind": "rust"})).unwrap()
    }
}

#[test]
fn phase_order_and_defaults_match_upstream() {
    assert_eq!(PHASE_ORDER, ["bootstrap", "build", "test"]);
}

#[test]
fn run_verify_executes_phases_and_reports_ok() {
    let td = make_project();
    let recipe = rust_recipe(None);
    let result = run_verify(td.path(), &recipe, None, 30.0, 5.0, true, None, true, None);
    assert!(result.ok());
    assert_eq!(result.recipe_name, "Rust project");
    // bootstrap (empty), build, test.
    let phases: Vec<&str> = result.phases.iter().map(|p| p.phase.as_str()).collect();
    assert_eq!(phases, vec!["build", "test"]);
    assert!(result.phases.iter().all(|p| p.ok()));
    assert_eq!(result.phases[0].output_tail.trim(), "building");
    assert!(result.readiness.is_none(), "skip_start = true");

    // The dict shape mirrors upstream's camelCase keys.
    let dict = result.to_dict();
    assert_eq!(dict["ok"], true);
    assert_eq!(dict["phases"][0]["phase"], "build");
    assert_eq!(dict["phases"][0]["timedOut"], false);
    assert_eq!(dict["readiness"], serde_json::Value::Null);
}

#[test]
fn failing_phase_stops_by_default_and_marks_not_ok() {
    let td = make_project();
    let mut recipe = rust_recipe(None);
    recipe.build = vec!["echo before".to_string(), "exit 3".to_string()];
    let result = run_verify(td.path(), &recipe, None, 30.0, 5.0, true, None, true, None);
    assert!(!result.ok());
    let commands: Vec<&str> = result.phases.iter().map(|p| p.command.as_str()).collect();
    // stop_on_failure: "exit 3" ran, but test phase never did.
    assert_eq!(commands, vec!["echo before", "exit 3"]);
    assert!(result.phases[1].exit_code == Some(3));
}

#[test]
fn stop_on_failure_false_runs_the_remaining_phases() {
    let td = make_project();
    let mut recipe = rust_recipe(None);
    recipe.build = vec!["exit 3".to_string()];
    let result = run_verify(td.path(), &recipe, None, 30.0, 5.0, true, None, false, None);
    let phases: Vec<&str> = result.phases.iter().map(|p| p.phase.as_str()).collect();
    assert_eq!(phases, vec!["build", "test"], "test still ran");
    assert!(!result.ok());
}

#[test]
fn phase_selection_and_timeout_paths() {
    let td = make_project();
    let recipe = rust_recipe(None);
    // Selecting only "test" skips build.
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["test"]),
        30.0,
        5.0,
        true,
        None,
        true,
        None,
    );
    let phases: Vec<&str> = result.phases.iter().map(|p| p.phase.as_str()).collect();
    assert_eq!(phases, vec!["test"]);

    // A command that outlives its timeout is killed and flagged.
    let mut recipe = rust_recipe(None);
    recipe.build = vec!["sleep 30".to_string()];
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["build"]),
        1.0,
        5.0,
        true,
        None,
        true,
        None,
    );
    let build = &result.phases[0];
    assert!(build.timed_out);
    assert_eq!(build.exit_code, None);
    assert!(!build.ok());
}

#[test]
fn start_phase_proves_readiness_and_tears_down() {
    let td = make_project();
    // A real HTTP server that answers the readiness probe, then blocks.
    let recipe = rust_recipe(Some(
        "python3 -m http.server 8123 --bind 127.0.0.1 2>/dev/null",
    ));
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["start"]),
        30.0,
        30.0,
        false,
        None,
        true,
        None,
    );
    let readiness = result
        .readiness
        .clone()
        .unwrap_or_else(|| panic!("no readiness: {:?}", result));
    assert!(readiness.ready, "{:?}", readiness.error);
    assert_eq!(readiness.status_code, Some(200));
    assert!(
        readiness.duration < 30.0,
        "readiness succeeded, no full wait"
    );
    assert!(result.ok());
}

#[test]
fn compose_recipe_refuses_live_containers() {
    // PARITY: the #103567 guard — a compose recipe with build/start
    // selected refuses when `docker compose ps` reports runners. The
    // probe is faked with a `docker` shim on PATH.
    let td = make_project();
    let bin = td.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    std::fs::write(bin.join("docker"), "#!/bin/sh\necho web-1\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("docker"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let mut recipe = rust_recipe(None);
    recipe.kind = "compose".to_string();
    recipe.build = vec!["docker compose build".to_string()];
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", bin.display(), old_path);
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = hermes_agent::verify::runner::run_verify(
        td.path(),
        &recipe,
        Some(&["build"]),
        30.0,
        5.0,
        false,
        None,
        true,
        None,
    );
    unsafe { std::env::set_var("PATH", &old_path) };
    assert!(!result.ok(), "live containers must refuse");
    assert_eq!(result.phases.len(), 1);
    assert_eq!(result.phases[0].phase, "build");
    assert_eq!(result.phases[0].exit_code, Some(1));
    assert!(
        result.phases[0]
            .output_tail
            .contains("already has running container(s): web-1"),
        "{}",
        result.phases[0].output_tail
    );
}

#[test]
fn compose_probe_clean_proceeds() {
    // No runners reported → the build phase runs normally.
    let td = make_project();
    let bin = td.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    std::fs::write(bin.join("docker"), "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("docker"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let mut recipe = rust_recipe(None);
    recipe.kind = "compose".to_string();
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", bin.display(), old_path);
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = hermes_agent::verify::runner::run_verify(
        td.path(),
        &recipe,
        Some(&["build"]),
        30.0,
        5.0,
        false,
        None,
        true,
        None,
    );
    unsafe { std::env::set_var("PATH", &old_path) };
    assert!(result.ok(), "{:?}", result.phases);
}

#[test]
fn start_phase_tail_includes_stderr() {
    // The start child merges stderr into the output tail (upstream
    // `stderr=STDOUT`); a stderr-only server still leaves diagnostics.
    let td = make_project();
    let recipe = rust_recipe(Some(
        r#"python3 -c "import sys, time; print('boot-err', file=sys.stderr); time.sleep(30)""#,
    ));
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["start"]),
        30.0,
        2.0,
        false,
        None,
        true,
        None,
    );
    let readiness = result.readiness.expect("readiness recorded");
    assert!(!readiness.ready, "nothing listens on 8123");
    assert!(
        readiness.output_tail.contains("boot-err"),
        "stderr merged into tail: {:?}",
        readiness.output_tail
    );
}

#[test]
fn readiness_probe_treats_any_http_response_as_up() {
    let td = make_project();
    // A plain http.server answers 404 for a missing readiness path —
    // upstream's HTTPError arm: the server answered, so it is up.
    let mut recipe = rust_recipe(Some(
        "python3 -m http.server 8124 --bind 127.0.0.1 2>/dev/null",
    ));
    recipe.readiness_path = "/missing-path-404".to_string();
    recipe.port = Some(8124);
    eprintln!("DBG calling run_verify");
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["start"]),
        30.0,
        30.0,
        false,
        None,
        true,
        None,
    );
    eprintln!("DBG result: {:?}", result);
    let readiness = result
        .readiness
        .clone()
        .unwrap_or_else(|| panic!("no readiness: {:?}", result));
    assert!(readiness.ready);
    assert_eq!(readiness.status_code, Some(404));
}

#[test]
fn start_failure_degrades_to_not_ready() {
    let td = make_project();
    // Nothing listens on the port: the readiness loop times out (short) and
    // the result is not ok. Distinct port: parallel tests each get their own.
    let recipe = rust_recipe(Some("sleep 100"));
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["start"]),
        30.0,
        2.0,
        false,
        Some(8126),
        true,
        None,
    );
    let readiness = result
        .readiness
        .clone()
        .unwrap_or_else(|| panic!("no readiness: {:?}", result));
    assert!(!readiness.ready);
    assert!(!result.ok());
}

#[test]
fn detection_plus_runner_end_to_end() {
    let td = tempfile::TempDir::new().unwrap();
    fs::write(td.path().join("Makefile"), "test:\n\techo make-tests-ok\n").unwrap();
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "make");
    let result = run_verify(
        td.path(),
        &recipe,
        Some(&["test"]),
        30.0,
        5.0,
        true,
        None,
        true,
        None,
    );
    assert!(result.ok());
    assert!(
        result.phases[0].output_tail.contains("make-tests-ok"),
        "make echoes the recipe line before running it"
    );
}
