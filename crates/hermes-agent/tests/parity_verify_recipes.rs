//! Parity tests for `agent/verify/recipes.py` @ 5d59366.
//!
//! Oracle: `tests/verify/test_recipes.py` (all classes) plus live
//! interpreter probes for coercion edge cases. The `environment.py`
//! manifest tests below predate the oracle split and stay green.
//! Tier: `unit`.

use std::fs;
use std::path::Path;

use serde_json::json;

use hermes_agent::verify::environment::{
    load_manifest, load_or_detect, manifest_path, save_manifest,
};
use hermes_agent::verify::recipes::{detect_package_manager, detect_recipe, Recipe};

fn write(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

// ── Recipe round-trip + tolerant loader ──────────────────────────────────

#[test]
fn recipe_to_dict_uses_grok_camel_case() {
    let recipe = Recipe {
        name: "Next.js".to_string(),
        kind: "nextjs".to_string(),
        bootstrap: vec!["npm install".to_string()],
        build: vec!["npm run build".to_string()],
        test: vec!["npm run test".to_string()],
        start: Some("npm run dev".to_string()),
        port: Some(3000),
        readiness_path: "/health".to_string(),
        evidence: vec!["Detected package.json".to_string()],
    };
    let dict = recipe.to_dict();
    assert_eq!(dict["readinessPath"], "/health");
    assert_eq!(dict["port"], 3000);
    assert_eq!(dict["name"], "Next.js");
    assert_eq!(Recipe::from_dict(&dict).unwrap(), recipe);
}

#[test]
fn oracle_lockfile_matrix_and_pnpm_priority() {
    // ORACLE: `TestPackageManagerDetection` — every lockfile maps, and
    // pnpm wins over yarn by candidate order.
    for (lockfile, manager) in [
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("uv.lock", "uv"),
        ("poetry.lock", "poetry"),
        ("Pipfile.lock", "pipenv"),
    ] {
        let td = tempfile::TempDir::new().unwrap();
        write(td.path(), lockfile, "");
        assert_eq!(
            detect_package_manager(td.path()).as_deref(),
            Some(manager),
            "{lockfile}"
        );
    }
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "pnpm-lock.yaml", "");
    write(td.path(), "yarn.lock", "");
    assert_eq!(detect_package_manager(td.path()).as_deref(), Some("pnpm"));
}

#[test]
fn oracle_node_recipes_per_case() {
    // ORACLE: `TestNodeDetection::test_nextjs_with_pnpm` — exact commands.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"dependencies": {"next": "14.0.0"},
            "scripts": {"dev": "next dev", "build": "next build", "test": "jest"}}"#,
    );
    write(td.path(), "pnpm-lock.yaml", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "nextjs");
    assert_eq!(recipe.bootstrap, vec!["pnpm install"]);
    assert_eq!(recipe.build, vec!["pnpm build"]);
    assert_eq!(recipe.test, vec!["pnpm test"]);
    assert_eq!(recipe.start.as_deref(), Some("pnpm dev"));
    assert_eq!(recipe.port, Some(3000));

    // ORACLE: `test_vite_with_yarn` — devDeps count, yarn runner.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"devDependencies": {"vite": "5.0.0"},
            "scripts": {"dev": "vite", "build": "vite build"}}"#,
    );
    write(td.path(), "yarn.lock", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "vite");
    assert_eq!(recipe.bootstrap, vec!["yarn install"]);
    assert_eq!(recipe.start.as_deref(), Some("yarn dev"));
    assert_eq!(recipe.port, Some(5173));

    // ORACLE: `test_bun_runner` — bun.lockb, `bun run start`, kind node.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"scripts": {"start": "node server.js", "build": "tsc"}}"#,
    );
    write(td.path(), "bun.lockb", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "node");
    assert_eq!(recipe.bootstrap, vec!["bun install"]);
    assert_eq!(recipe.start.as_deref(), Some("bun run start"));

    // ORACLE: `test_generic_node_defaults_to_npm` — no lockfile, no start.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"scripts": {"test": "mocha"}}"#,
    );
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.bootstrap, vec!["npm install"]);
    assert_eq!(recipe.test, vec!["npm run test"]);
    assert_eq!(recipe.start, None);
    assert_eq!(recipe.port, None);

    // ORACLE: `test_port_inferred_from_start_command`.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"scripts": {"dev": "node server.js --port 4111"}}"#,
    );
    assert_eq!(detect_recipe(td.path()).unwrap().port, Some(4111));

    // ORACLE: `test_cra`.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"dependencies": {"react-scripts": "5.0"},
            "scripts": {"start": "react-scripts start"}}"#,
    );
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "cra");
    assert_eq!(recipe.port, Some(3000));

    // ORACLE: `test_malformed_package_json_falls_through` — bad JSON is
    // not a node project; detection continues to Go.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "package.json", "{not json");
    write(td.path(), "go.mod", "module x\n");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "go");
}

#[test]
fn oracle_python_recipes_per_case() {
    // ORACLE: `test_django_via_manage_py`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "manage.py", "");
    write(td.path(), "requirements.txt", "django\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "django");
    assert_eq!(recipe.test, vec!["python manage.py test"]);
    assert_eq!(recipe.port, Some(8000));
    assert!(recipe.start.as_deref().unwrap().contains("runserver"));

    // ORACLE: `test_fastapi_uvicorn` — main.py present → main:app.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "fastapi\nuvicorn\n");
    write(td.path(), "main.py", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "fastapi");
    assert!(recipe
        .start
        .as_deref()
        .unwrap()
        .starts_with("uvicorn main:app"));
    assert_eq!(recipe.port, Some(8000));

    // ORACLE: `test_flask`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "flask\n");
    write(td.path(), "app.py", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "flask");
    assert_eq!(recipe.port, Some(5000));

    // ORACLE: `test_generic_python_uv`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "pyproject.toml", "[project]\nname='x'\n");
    write(td.path(), "uv.lock", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "python");
    assert_eq!(recipe.bootstrap, vec!["uv sync"]);

    // ORACLE: `test_generic_python_pyproject_editable_install`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "pyproject.toml", "[project]\nname='x'\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.bootstrap, vec!["pip install -e ."]);
    assert_eq!(recipe.test, vec!["python -m unittest discover"]);

    // ORACLE: `test_pytest_when_tests_dir`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "requests\n");
    fs::create_dir(td.path().join("tests")).unwrap();
    assert_eq!(detect_recipe(td.path()).unwrap().test, vec!["pytest"]);
}

#[test]
fn oracle_other_ecosystems_per_case() {
    // ORACLE: `test_go` — main.go present → `go run .`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "go.mod", "module example.com/x\n");
    write(td.path(), "main.go", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "go");
    assert_eq!(recipe.build, vec!["go build ./..."]);
    assert_eq!(recipe.start.as_deref(), Some("go run ."));

    // ORACLE: `test_rust` + `test_rust_library_has_no_start`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Cargo.toml", "[package]\nname='x'\n");
    write(td.path(), "src/main.rs", "fn main() {}");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "rust");
    assert_eq!(recipe.start.as_deref(), Some("cargo run"));
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Cargo.toml", "[package]\nname='x'\n");
    assert_eq!(detect_recipe(td.path()).unwrap().start, None);

    // ORACLE: `test_maven`, `test_gradle_wrapper`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "pom.xml", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "maven");
    assert_eq!(recipe.build, vec!["mvn package"]);
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "build.gradle", "");
    write(td.path(), "gradlew", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "gradle");
    assert_eq!(recipe.build, vec!["./gradlew build"]);

    // ORACLE: `test_makefile` — exact pick per phase.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "Makefile",
        "install:\n\tpip install .\nbuild:\n\tmake -C src\ntest:\n\tpytest\nrun:\n\t./app\n",
    );
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "make");
    assert_eq!(recipe.bootstrap, vec!["make install"]);
    assert_eq!(recipe.build, vec!["make build"]);
    assert_eq!(recipe.test, vec!["make test"]);
    assert_eq!(recipe.start.as_deref(), Some("make run"));

    // ORACLE: `test_docker_compose`, `test_empty_dir_returns_none`.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "docker-compose.yml", "services: {}\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "compose");
    assert_eq!(recipe.start.as_deref(), Some("docker compose up"));
    let td = tempfile::TempDir::new().unwrap();
    assert!(detect_recipe(td.path()).is_none());
}

#[test]
fn oracle_from_dict_per_case() {
    // ORACLE: `test_roundtrip` — defaults survive the camelCase loop.
    let recipe = Recipe {
        name: "X".to_string(),
        kind: "node".to_string(),
        bootstrap: vec!["npm install".to_string()],
        start: Some("npm run dev".to_string()),
        port: Some(3000),
        ..Recipe::from_dict(&json!({"name": "X"})).unwrap()
    };
    assert_eq!(Recipe::from_dict(&recipe.to_dict()).unwrap(), recipe);

    // ORACLE: `test_tolerates_garbage` — null, list, nameless all reject.
    assert!(Recipe::from_dict(&json!(null)).is_none());
    assert!(Recipe::from_dict(&json!([])).is_none());
    assert!(Recipe::from_dict(&json!({"kind": "x"})).is_none());
    assert!(Recipe::from_dict(&json!({"name": "  "})).is_none());

    // ORACLE: `test_grok_style_keys` — every alias at once.
    let recipe = Recipe::from_dict(&json!({
        "appLabel": "Next.js",
        "appKind": "nextjs",
        "installCommands": ["npm install"],
        "buildCommands": ["npm run build"],
        "testCommands": ["npm test"],
        "startCommand": "npm run dev",
        "startPort": "3000",
    }))
    .unwrap();
    assert_eq!(recipe.name, "Next.js");
    assert_eq!(recipe.port, Some(3000));
    assert_eq!(recipe.bootstrap, vec!["npm install"]);

    // ORACLE: `test_invalid_port_dropped`.
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": "not-a-port"}))
            .unwrap()
            .port,
        None
    );
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": 99999999}))
            .unwrap()
            .port,
        None
    );

    // ORACLE: `test_bad_readiness_path_normalized`.
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "readinessPath": "health"}))
            .unwrap()
            .readiness_path,
        "/"
    );
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "readinessPath": "/health"}))
            .unwrap()
            .readiness_path,
        "/health"
    );
}

#[test]
fn from_dict_accepts_grok_aliases_and_rejects_blank_names() {
    let aliased = json!({
        "appLabel": "My App",
        "appKind": "web",
        "installCommands": "pip install -e .",
        "buildCommands": ["make build", ""],
        "testCommands": ["make test"],
        "startCommand": "  ./run.sh  ",
        "startPort": "8080",
        "readinessPath": "healthz",   // must start with "/" -> default
    });
    let recipe = Recipe::from_dict(&aliased).unwrap();
    assert_eq!(recipe.name, "My App");
    assert_eq!(recipe.kind, "web");
    assert_eq!(recipe.bootstrap, vec!["pip install -e ."]);
    assert_eq!(recipe.build, vec!["make build"], "blank entries drop");
    assert_eq!(recipe.start.as_deref(), Some("./run.sh"));
    assert_eq!(recipe.port, Some(8080), "numeric strings coerce");
    assert_eq!(recipe.readiness_path, "/");

    // No name at all -> rejected.
    assert!(Recipe::from_dict(&json!({"kind": "web"})).is_none());
    // Blank name -> rejected.
    assert!(Recipe::from_dict(&json!({"name": "   "})).is_none());
    // Non-dict -> rejected.
    assert!(Recipe::from_dict(&json!([1])).is_none());
    // Out-of-range ports rejected.
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": 99999}))
            .unwrap()
            .port,
        None
    );
}

#[test]
fn from_dict_bool_port_matches_upstream_truthiness() {
    // Oracle-verified: `port True` is truthy, not an int instance check —
    // upstream keeps `True` as the port (`0 < True < 65536`).
    let recipe = Recipe::from_dict(&json!({"name": "x", "port": true})).unwrap();
    assert_eq!(recipe.port, Some(1));
}

#[test]
fn from_dict_whitespace_port_string_coerces() {
    // Oracle-verified: `" 3000 "` strips then digit-checks → 3000, while
    // `"+3000"` and `"0"` fail the digit/range gates → None.
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": " 3000 "}))
            .unwrap()
            .port,
        Some(3000)
    );
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": "+3000"}))
            .unwrap()
            .port,
        None
    );
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "port": "0"}))
            .unwrap()
            .port,
        None
    );
}

#[test]
fn from_dict_blank_kind_and_readiness_fall_back() {
    // Oracle-verified: whitespace `kind` → "unknown"; empty/None
    // readiness → "/"; blank `start` → None.
    let recipe = Recipe::from_dict(&json!({"name": "x", "kind": "  "})).unwrap();
    assert_eq!(recipe.kind, "unknown");
    for raw in [
        json!({"name": "x", "readinessPath": ""}),
        json!({"name": "x", "readinessPath": null}),
    ] {
        assert_eq!(Recipe::from_dict(&raw).unwrap().readiness_path, "/");
    }
    assert_eq!(
        Recipe::from_dict(&json!({"name": "x", "start": "  "}))
            .unwrap()
            .start,
        None
    );
}

#[test]
fn from_dict_scalar_commands_and_mixed_evidence() {
    // Oracle-verified: a bare-string command list coerces to one entry,
    // non-string commands drop; evidence keeps dupes but strips.
    let recipe = Recipe::from_dict(&json!({
        "name": "x",
        "bootstrap": "npm i",
        "build": 5,
        "evidence": ["a", 1, null, " a "],
    }))
    .unwrap();
    assert_eq!(recipe.bootstrap, vec!["npm i"]);
    assert!(recipe.build.is_empty());
    assert_eq!(recipe.evidence, vec!["a", "a"]);
    // Oracle-verified: non-string `name` rejects the recipe; aliased
    // names strip.
    assert!(Recipe::from_dict(&json!({"name": 5})).is_none());
    let recipe = Recipe::from_dict(&json!({"appLabel": " A ", "appKind": " k "})).unwrap();
    assert_eq!(recipe.name, "A");
    assert_eq!(recipe.kind, "k");
}

#[test]
fn from_dict_float_ports_reject_and_alias_order_is_first_wins() {
    // Oracle-verified: floats are not `int` instances upstream → None
    // (even integral `3000.0`); `False` is falsy (`or`-chain skips it).
    for raw in [
        json!({"name": "x", "port": 3000.9}),
        json!({"name": "x", "port": 3000.0}),
        json!({"name": "x", "port": false}),
    ] {
        assert_eq!(Recipe::from_dict(&raw).unwrap().port, None, "{raw}");
    }
    // Oracle-verified: canonical keys precede grok aliases in every
    // `or`-chain (`name`/`kind`/`start`/`port`/`readinessPath`/`bootstrap`
    // first-wins).
    let recipe = Recipe::from_dict(&json!({
        "name": "first", "appLabel": "second",
        "kind": "first", "appKind": "second",
        "start": "first", "startCommand": "second",
        "port": 100, "startPort": 200,
        "readinessPath": "/a", "readiness_path": "/b",
        "bootstrap": "a", "installCommands": "b",
    }))
    .unwrap();
    assert_eq!(recipe.name, "first");
    assert_eq!(recipe.kind, "first");
    assert_eq!(recipe.start.as_deref(), Some("first"));
    assert_eq!(recipe.port, Some(100));
    assert_eq!(recipe.readiness_path, "/a");
    assert_eq!(recipe.bootstrap, vec!["a"]);
    // Oracle-verified: bare-string evidence coerces; bool ports round-trip
    // through `to_dict` (`True` serializes and reloads as `Some(1)`).
    let recipe = Recipe::from_dict(&json!({"name": "x", "evidence": "solo"})).unwrap();
    assert_eq!(recipe.evidence, vec!["solo"]);
    let recipe = Recipe {
        name: "x".to_string(),
        port: Some(1),
        ..Recipe::from_dict(&json!({"name": "x"})).unwrap()
    };
    let dict = recipe.to_dict();
    assert_eq!(dict["port"], 1);
    assert_eq!(Recipe::from_dict(&dict).unwrap(), recipe);
}

#[test]
fn infer_port_regex_boundaries_match_oracle() {
    // Oracle-verified: `PORT:3000` / `port 3000` / `--port=3000` never
    // match (flag needs whitespace, env needs `=` right after PORT);
    // a 5-digit `--port 30001` matches fully, 6 digits match the first 5.
    let td = tempfile::TempDir::new().unwrap();
    for (script, port) in [
        ("node s --port 30001", 30001),
        ("node s --port 300001", 30000),
    ] {
        write(
            td.path(),
            "package.json",
            &format!(r#"{{"scripts": {{"dev": "{script}"}}}}"#),
        );
        assert_eq!(
            detect_recipe(td.path()).unwrap().port,
            Some(port),
            "{script}"
        );
    }
}

#[test]
fn detect_node_non_string_script_start_fails_open() {
    // DOCUMENTED DIVERGENCE (see module docs): `"dev": 5` selects the dev
    // start entry, then upstream crashes in `_infer_port_from_command`
    // (TypeError on int). The port fails open to None here — pinned so a
    // strictness change fails loudly.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"scripts": {"dev": 5, "build": "x"}}"#,
    );
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "node");
    assert_eq!(recipe.start, None);
    assert_eq!(recipe.port, None);
}

#[test]
fn detect_recipe_package_json_wins_over_all_runtimes() {
    // Oracle-verified: package.json (even script-less) beats Python, Go,
    // Makefile and compose manifests.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "package.json", r#"{"scripts": {"dev": "x"}}"#);
    write(td.path(), "go.mod", "module x");
    write(td.path(), "pyproject.toml", "[p]");
    write(td.path(), "Makefile", "a:\n");
    write(td.path(), "compose.yaml", "{}");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "node");
    // Oracle-verified: a JSON list is not a manifest — falls through.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "package.json", "[]");
    assert!(detect_recipe(td.path()).is_none());
}

#[test]
fn detect_node_framework_order_and_script_subtleties() {
    // Oracle-verified: `next` beats `vite` in framework order; a single
    // `@remix-run/react` dep is enough for remix.
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"dependencies": {"next": "1", "vite": "1"}, "scripts": {}}"#,
    );
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "nextjs");
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"dependencies": {"@remix-run/react": "1"}, "scripts": {}}"#,
    );
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "remix");

    // Oracle-verified: non-dict `scripts`/`dependencies` degrade to empty
    // (node default, no start, `(none)` evidence).
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "package.json", r#"{"scripts": "nope"}"#);
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "node");
    assert_eq!(recipe.start, None);
    assert!(recipe.evidence.iter().any(|e| e.contains("(none)")));
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"dependencies": "nope", "scripts": {}}"#,
    );
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "node");
}

#[test]
fn detect_node_port_forms_and_case_folding() {
    // Oracle-verified: `PORT=` env form and `-p` short flag both infer.
    for (script, port) in [
        ("PORT=4000 node s", 4000),
        ("node s -p 4111", 4111),
        ("--port 3000 --port 4000", 3000),
    ] {
        let td = tempfile::TempDir::new().unwrap();
        write(
            td.path(),
            "package.json",
            &format!(r#"{{"scripts": {{"dev": "{script}"}}}}"#),
        );
        assert_eq!(
            detect_recipe(td.path()).unwrap().port,
            Some(port),
            "{script}"
        );
    }
    // Oracle-verified: one-digit PORT never matches (`\d{2,5}`).
    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "package.json",
        r#"{"scripts": {"dev": "PORT=1 node s"}}"#,
    );
    assert_eq!(detect_recipe(td.path()).unwrap().port, None);
}

#[test]
fn detect_python_precedence_and_setup_py_gate() {
    // Oracle-verified: Django by dependency alone (no manage.py), and
    // setup.py alone opens the generic gate with the requirements default.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "Django\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "django");
    assert!(recipe.evidence.iter().any(|e| e.contains("dependency")));
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "setup.py", "x");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "python");
    assert_eq!(recipe.bootstrap, vec!["pip install -r requirements.txt"]);
    assert_eq!(recipe.test, vec!["python -m unittest discover"]);

    // Oracle-verified: main.py wins for FastAPI module choice; app.py is
    // the Flask fallback when only main.py exists.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "fastapi");
    write(td.path(), "app.py", "");
    write(td.path(), "main.py", "");
    assert!(detect_recipe(td.path())
        .unwrap()
        .start
        .as_deref()
        .unwrap()
        .starts_with("uvicorn main:app"));
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "requirements.txt", "flask");
    write(td.path(), "main.py", "");
    assert!(detect_recipe(td.path())
        .unwrap()
        .start
        .as_deref()
        .unwrap()
        .contains("main.py"));
}

#[test]
fn detect_make_gradle_compose_subtleties() {
    // Oracle-verified: `build.gradle.kts` counts; spaced target names do
    // not (`"foo bar:"` matches nothing); duplicate targets list twice.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "build.gradle.kts", "");
    write(td.path(), "gradlew", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "gradle");
    assert_eq!(recipe.build, vec!["./gradlew build"]);
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Makefile", "foo bar:\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert!(recipe.evidence[1].contains("(none)"));
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Makefile", "build: x\nbuild: y\n");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.build, vec!["make build"]);
    assert!(recipe.evidence[1].contains("build, build"));
    // Oracle-verified: empty Makefile still yields a make recipe; Go
    // outranks Rust; `compose.yml` is a compose root.
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Makefile", "");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "make");
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "go.mod", "m");
    write(td.path(), "Cargo.toml", "x");
    write(td.path(), "src/main.rs", "");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "go");
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "compose.yml", "{}");
    assert!(detect_recipe(td.path())
        .unwrap()
        .evidence
        .iter()
        .any(|e| e.contains("compose.yml")));
}

// ── detection ────────────────────────────────────────────────────────────

#[test]
fn package_manager_detection_by_lockfile_priority() {
    let td = tempfile::TempDir::new().unwrap();
    let root = td.path();
    assert_eq!(detect_package_manager(root), None);
    write(root, "package-lock.json", "{}");
    assert_eq!(detect_package_manager(root).as_deref(), Some("npm"));
    // pnpm outranks npm by candidate order.
    write(root, "pnpm-lock.yaml", "");
    assert_eq!(detect_package_manager(root).as_deref(), Some("pnpm"));
}

#[test]
fn node_recipe_detects_framework_and_scripts() {
    let td = tempfile::TempDir::new().unwrap();
    let root = td.path();
    write(root, "pnpm-lock.yaml", "");
    write(
        root,
        "package.json",
        r#"{
            "name": "app",
            "scripts": {"dev": "next dev --port 3005", "build": "next build", "lint": "next lint"},
            "dependencies": {"next": "15.0.0"}
        }"#,
    );
    let recipe = detect_recipe(root).unwrap();
    assert_eq!(recipe.kind, "nextjs");
    assert_eq!(recipe.name, "Next.js");
    assert_eq!(recipe.bootstrap, vec!["pnpm install"]);
    assert_eq!(recipe.start.as_deref(), Some("pnpm dev"));
    assert_eq!(recipe.port, Some(3005), "inferred from the dev script body");
    assert_eq!(recipe.build, vec!["pnpm build"], "typecheck script absent");
    assert_eq!(recipe.test, vec!["pnpm lint"]);
    assert!(recipe.evidence.iter().any(|e| e.contains("pnpm")));
    assert!(recipe
        .evidence
        .iter()
        .any(|e| e.contains("Detected package.json")));
}

#[test]
fn python_recipe_prefers_django_then_fastapi() {
    let td = tempfile::TempDir::new().unwrap();
    let root = td.path();
    write(root, "manage.py", "");
    write(root, "requirements.txt", "django\n");
    let recipe = detect_recipe(root).unwrap();
    assert_eq!(recipe.kind, "django");
    assert_eq!(recipe.port, Some(8000));
    assert!(recipe.evidence.iter().any(|e| e.contains("manage.py")));

    // FastAPI without main.py/app.py defaults to main:app.
    let td2 = tempfile::TempDir::new().unwrap();
    write(td2.path(), "requirements.txt", "fastapi\nuvicorn\n");
    let recipe = detect_recipe(td2.path()).unwrap();
    assert_eq!(recipe.kind, "fastapi");
    assert_eq!(
        recipe.start.as_deref(),
        Some("uvicorn main:app --host 0.0.0.0 --port 8000")
    );
    // app.py wins over the default module.
    write(td2.path(), "app.py", "");
    let recipe = detect_recipe(td2.path()).unwrap();
    assert!(recipe.start.as_deref().unwrap().contains("app:app"));
}

#[test]
fn go_rust_java_make_and_compose_fallbacks() {
    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "go.mod", "module x\n");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "go");

    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "Cargo.toml", "");
    write(td.path(), "src/main.rs", "fn main() {}");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "rust");
    assert_eq!(recipe.start.as_deref(), Some("cargo run"));

    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "pom.xml", "");
    assert_eq!(detect_recipe(td.path()).unwrap().kind, "maven");

    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "build.gradle", "");
    write(td.path(), "gradlew", "");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.build, vec!["./gradlew build"]);

    let td = tempfile::TempDir::new().unwrap();
    write(
        td.path(),
        "Makefile",
        "install:\n\ttrue\nbuild:\n\ttrue\ntest:\n\ttrue\n.PHONY: build\n",
    );
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "make");
    assert_eq!(recipe.bootstrap, vec!["make install"]);
    assert_eq!(recipe.build, vec!["make build"]);
    assert_eq!(recipe.test, vec!["make test"]);
    // NOTE: `.PHONY: build` DOES count as a target — the upstream
    // character class includes dots (`[A-Za-z0-9_.-]+`), verified against
    // the Python oracle.
    assert!(recipe.evidence[1].contains(".PHONY"));

    let td = tempfile::TempDir::new().unwrap();
    write(td.path(), "compose.yaml", "services: {}");
    let recipe = detect_recipe(td.path()).unwrap();
    assert_eq!(recipe.kind, "compose");
    assert_eq!(recipe.start.as_deref(), Some("docker compose up"));

    // Nothing recognizable -> None.
    let td = tempfile::TempDir::new().unwrap();
    assert!(detect_recipe(td.path()).is_none());
}

// ── environment manifest ─────────────────────────────────────────────────

#[test]
fn manifest_round_trip_and_tolerant_load() {
    let td = tempfile::TempDir::new().unwrap();
    let root = td.path();
    let recipe = Recipe {
        name: "Rust project".to_string(),
        kind: "rust".to_string(),
        test: vec!["cargo test".to_string()],
        ..Recipe::from_dict(&json!({"name": "Rust project", "kind": "rust"})).unwrap()
    };
    let path = save_manifest(root, &recipe).unwrap();
    assert_eq!(path, manifest_path(root));
    assert!(path.to_string_lossy().contains(".hermes/environment.json"));

    let loaded = load_manifest(root).unwrap();
    assert_eq!(loaded, recipe);

    // Corrupt manifest degrades to None (fresh detection), file untouched.
    fs::write(manifest_path(root), "{broken").unwrap();
    assert!(load_manifest(root).is_none());
    // Non-dict manifest too.
    fs::write(manifest_path(root), "[1]").unwrap();
    assert!(load_manifest(root).is_none());
}

#[test]
fn load_or_detect_prefers_the_manifest() {
    let td = tempfile::TempDir::new().unwrap();
    let root = td.path();
    write(root, "Cargo.toml", "");
    let (recipe, source) = load_or_detect(root);
    assert_eq!(source, "detected");
    assert_eq!(recipe.unwrap().kind, "rust");

    // A saved manifest — even for a different stack — wins over detection.
    let saved_recipe = Recipe {
        name: "Custom".to_string(),
        kind: "custom".to_string(),
        ..Recipe::from_dict(&json!({"name": "Custom", "kind": "custom"})).unwrap()
    };
    save_manifest(root, &saved_recipe).unwrap();
    let (recipe, source) = load_or_detect(root);
    assert_eq!(source, "manifest");
    assert_eq!(recipe.unwrap().kind, "custom");
}

// ── `agent/verify/__init__.py` re-export surface ─────────────────────────

#[test]
fn init_reexport_surface_is_complete() {
    // Upstream `agent/verify/__init__.py` re-exports exactly these names;
    // each must resolve through the package root.
    let _recipe: fn(Option<&str>) -> Option<hermes_agent::verify::Recipe> =
        |_| hermes_agent::verify::recipes::detect_recipe(std::path::Path::new("."));
    let _: Option<Recipe> =
        hermes_agent::verify::recipes::detect_recipe(Path::new("/definitely-not-here"));
    let _: Option<String> =
        hermes_agent::verify::recipes::detect_package_manager(Path::new("/definitely-not-here"));
    let _: Option<Recipe> = hermes_agent::verify::load_manifest(Path::new("/definitely-not-here"));
    let _: std::path::PathBuf = hermes_agent::verify::manifest_path(Path::new("/p"));
    let _: (Option<Recipe>, &'static str) =
        hermes_agent::verify::load_or_detect(Path::new("/definitely-not-here"));
    // run_verify / PhaseResult / ReadinessResult / VerifyResult resolve via
    // the runner module (used throughout this file's siblings).
    let _ = hermes_agent::verify::run_verify;
}
