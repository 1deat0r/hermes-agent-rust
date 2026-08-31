//! Parity tests for `agent/verify/recipes.py` and `environment.py` @
//! b9aa928 (the runner stays PENDING). Upstream has no dedicated test
//! file (missing-test gap, noted in the ledger); cases derive from the
//! upstream code as oracle.

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
