//! Static run-recipe detection for project verification.
//!
//! PARITY: `agent/verify/recipes.py` @ 5d59366 (whole module). Ported
//! nearly 1:1 from superagent-ai/grok-cli `src/verify/recipes.ts`; grok's
//! detection order and command choices are the oracle.
//!
//! Layer ownership vs `agent.coding_context`: that module owns the *cheap
//! prompt-time facts*; this module owns the *deep runtime recipe* —
//! framework identification, bootstrap/build/test command inference, and
//! the start command, port, and readiness path.
//!
//! PORT SEAMS (documented divergences):
//! - A non-string `scripts[start]` entry (e.g. `"dev": 5`) returns `None`
//!   for start/port here; upstream crashes with `TypeError` in
//!   `_infer_port_from_command`. No caller feeds non-string scripts
//!   (they come from parsed package.json authorial content), and failing
//!   open matches every other malformed-manifest path in this module.
//!   Pinned in the parity suite as `None` so a strictness change fails
//!   loudly.

use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};

/// A runnable verification recipe for a project.
///
/// Mirrors grok-cli's `VerifyRecipe` with a scoped field set: `name` is
/// the human label (grok's `appLabel`), `kind` the detector id (grok's
/// `appKind`), and command lists are shell strings executed in the project
/// root.
///
/// PARITY: `Recipe` (upstream lines 26-77).
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub name: String,
    pub kind: String,
    pub bootstrap: Vec<String>,
    pub build: Vec<String>,
    pub test: Vec<String>,
    pub start: Option<String>,
    pub port: Option<i64>,
    pub readiness_path: String,
    pub evidence: Vec<String>,
}

impl Recipe {
    /// PARITY: `to_dict` (upstream lines 42-47) — camelCase keys.
    pub fn to_dict(&self) -> Value {
        json!({
            "name": self.name,
            "kind": self.kind,
            "bootstrap": self.bootstrap,
            "build": self.build,
            "test": self.test,
            "start": self.start,
            "port": self.port,
            "readinessPath": self.readiness_path,
            "evidence": self.evidence,
        })
    }

    /// Tolerant loader mirroring grok's `normalizeVerifyRecipe`.
    ///
    /// PARITY: `from_dict` (upstream lines 49-77): accepts both the grok
    /// aliases (`appLabel`, `appKind`, `installCommands`, `buildCommands`,
    /// `testCommands`, `startCommand`, `startPort`) and the canonical
    /// names; a missing/blank name rejects the whole recipe; the port must
    /// be 0 < port < 65536; the readiness path must start with `/`.
    pub fn from_dict(raw: &Value) -> Option<Recipe> {
        let Some(map) = raw.as_object() else {
            return None;
        };
        let get = |keys: &[&str]| -> Option<&Value> {
            keys.iter()
                .find_map(|k| map.get(*k))
                .filter(|v| !v.is_null())
        };
        let as_str = |v: &Value| -> Option<String> {
            v.as_str()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let name = get(&["name", "appLabel"]).and_then(as_str)?;
        let kind = get(&["kind", "appKind"])
            .and_then(as_str)
            .unwrap_or_else(|| "unknown".to_string());

        let as_strings = |value: Option<&Value>| -> Vec<String> {
            match value {
                Some(Value::String(s)) if !s.trim().is_empty() => {
                    vec![s.trim().to_string()]
                }
                Some(Value::Array(items)) => items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                _ => Vec::new(),
            }
        };

        let start_value =
            get(&["start", "startCommand"]).and_then(|v| v.as_str().map(str::to_string));
        let start = start_value
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let port_raw = get(&["port", "startPort"]);
        let port = match port_raw {
            // PARITY: `isinstance(port_raw, int)` — Python `bool` IS an
            // `int` subclass, so `port True` is kept (`0 < True < 65536`).
            // `as_i64` covers both; floats are not ints upstream → None.
            Some(Value::Bool(true)) => Some(1),
            Some(Value::Bool(false)) => None,
            Some(Value::Number(n)) if n.is_i64() || n.is_u64() => {
                let port = n.as_i64()?;
                if 0 < port && port < 65536 {
                    Some(port)
                } else {
                    None
                }
            }
            Some(Value::Number(_)) => None,
            Some(Value::String(s))
                if !s.trim().is_empty() && s.trim().chars().all(|c| c.is_ascii_digit()) =>
            {
                let candidate: i64 = s.trim().parse().ok()?;
                if 0 < candidate && candidate < 65536 {
                    Some(candidate)
                } else {
                    None
                }
            }
            _ => None,
        };

        let readiness_value = get(&["readinessPath", "readiness_path"]).and_then(|v| v.as_str());
        let readiness_path = match readiness_value {
            Some(r) if r.starts_with('/') => r.to_string(),
            _ => "/".to_string(),
        };

        Some(Recipe {
            name: name.trim().to_string(),
            kind: kind.trim().to_string(),
            bootstrap: as_strings(get(&["bootstrap", "installCommands"])),
            build: as_strings(get(&["build", "buildCommands"])),
            test: as_strings(get(&["test", "testCommands"])),
            start,
            port,
            readiness_path,
            evidence: as_strings(get(&["evidence"])),
        })
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// PARITY: `_read_text` (upstream lines 80-84).
fn read_text(root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(root.join(name)).ok()
}

/// PARITY: `_read_package_json` (upstream lines 87-92).
fn read_package_json(root: &Path) -> Option<Value> {
    let raw = read_text(root, "package.json")?;
    let parsed: Value = serde_json::from_str(&raw).ok()?;
    if parsed.is_object() {
        Some(parsed)
    } else {
        None
    }
}

/// Lockfile-based package-manager detection (grok's detectPackageManager).
///
/// PARITY: `detect_package_manager` (upstream lines 108-110).
pub fn detect_package_manager(root: &Path) -> Option<String> {
    let candidates = [
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("uv.lock", "uv"),
        ("poetry.lock", "poetry"),
        ("Pipfile.lock", "pipenv"),
    ];
    for (filename, manager) in candidates {
        if root.join(filename).exists() {
            return Some(manager.to_string());
        }
    }
    None
}

/// Port inference from a start command (grok's inferPortFromCommand).
///
/// PARITY: `_infer_port_from_command` (upstream lines 113-118).
fn infer_port_from_command(command: Option<&str>) -> Option<i64> {
    static FLAG_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?:--port|-p)\s+(\d{2,5})").expect("port flag re"));
    static ENV_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\bPORT=(\d{2,5})\b").expect("port env re"));

    let command = command?;
    if let Some(caps) = FLAG_RE.captures(command) {
        if let Ok(port) = caps[1].parse::<i64>() {
            return Some(port);
        }
    }
    ENV_RE
        .captures(command)
        .and_then(|caps| caps[1].parse::<i64>().ok())
}

/// PARITY: `_dedupe` (upstream lines 121-123) — first-occurrence order,
/// stripped, blank values dropped.
fn dedupe(values: Vec<Option<String>>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for value in values.into_iter().flatten() {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() && !seen.contains(&trimmed) {
            seen.push(trimmed);
        }
    }
    seen
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// PARITY: `_script_runner` (upstream lines 137-138).
fn script_runner(package_manager: Option<&str>, entry: &str) -> String {
    match package_manager {
        Some("pnpm") => format!("pnpm {entry}"),
        Some("bun") => format!("bun run {entry}"),
        Some("yarn") => format!("yarn {entry}"),
        _ => format!("npm run {entry}"),
    }
}

/// PARITY: `_detect_node_recipe` (upstream lines 141-169).
fn detect_node_recipe(root: &Path, pkg: &Value) -> Recipe {
    let scripts: serde_json::Map<String, Value> = pkg
        .get("scripts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut deps = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(section) = pkg.get(key).and_then(Value::as_object) {
            for (k, v) in section {
                deps.insert(k.clone(), v.clone());
            }
        }
    }

    let package_manager = detect_package_manager(root);

    let (kind, label, default_port) = if deps.contains_key("next") {
        ("nextjs", "Next.js", Some(3000))
    } else if deps.contains_key("@sveltejs/kit") {
        ("sveltekit", "SvelteKit", Some(5173))
    } else if deps.contains_key("astro") {
        ("astro", "Astro", Some(4321))
    } else if deps.contains_key("@remix-run/dev") || deps.contains_key("@remix-run/react") {
        ("remix", "Remix", Some(3000))
    } else if deps.contains_key("react-scripts") {
        ("cra", "Create React App", Some(3000))
    } else if deps.contains_key("vite") {
        ("vite", "Vite", Some(5173))
    } else {
        ("node", "Node.js app", None)
    };

    let install = match package_manager.as_deref() {
        Some("pnpm") => "pnpm install",
        Some("bun") => "bun install",
        Some("yarn") => "yarn install",
        _ => "npm install",
    };

    let script_str = |name: &str| scripts.get(name).and_then(Value::as_str);
    let start_script = if script_str("dev").is_some() {
        "dev"
    } else if script_str("start").is_some() {
        "start"
    } else {
        ""
    };
    let start_body = if start_script.is_empty() {
        None
    } else {
        script_str(start_script)
    };
    let start = if start_script.is_empty() {
        None
    } else {
        Some(script_runner(package_manager.as_deref(), start_script))
    };
    // `port = _infer_port_from_command(start_body) or default_port if start
    // else None`.
    let port = if start.is_some() {
        infer_port_from_command(start_body).or(default_port)
    } else {
        None
    };

    let build = dedupe(
        ["build", "typecheck"]
            .iter()
            .filter(|s| script_str(s).is_some())
            .map(|s| Some(script_runner(package_manager.as_deref(), s)))
            .collect(),
    );
    let test = dedupe(
        ["test", "check", "lint"]
            .iter()
            .filter(|s| script_str(s).is_some())
            .map(|s| Some(script_runner(package_manager.as_deref(), s)))
            .collect(),
    );

    let script_list = scripts.keys().cloned().collect::<Vec<_>>().join(", ");
    let script_list = if script_list.is_empty() {
        "(none)".to_string()
    } else {
        script_list
    };
    let mut evidence = vec![Some("Detected package.json".to_string())];
    evidence.push(
        package_manager
            .as_ref()
            .map(|pm| format!("Package manager: {pm}")),
    );
    evidence.push(Some(format!("Scripts: {script_list}")));

    Recipe {
        name: label.to_string(),
        kind: kind.to_string(),
        bootstrap: vec![install.to_string()],
        build,
        test,
        start: start.clone(),
        port: if start.is_some() { port } else { None },
        readiness_path: "/".to_string(),
        evidence: dedupe(evidence),
    }
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

/// PARITY: `_detect_python_recipe` (upstream lines 175-214).
fn detect_python_recipe(root: &Path) -> Option<Recipe> {
    let pyproject = read_text(root, "pyproject.toml");
    let requirements = read_text(root, "requirements.txt");
    let manage_py = root.join("manage.py").exists();
    let has_setup_py = root.join("setup.py").exists();
    if pyproject.is_none() && requirements.is_none() && !manage_py && !has_setup_py {
        return None;
    }

    let lower = format!(
        "{}\n{}",
        pyproject.as_deref().unwrap_or(""),
        requirements.as_deref().unwrap_or("")
    )
    .to_lowercase();
    let package_manager = detect_package_manager(root);
    let is_django = manage_py || lower.contains("django");
    let is_fastapi = lower.contains("fastapi") || lower.contains("uvicorn");
    let is_flask = lower.contains("flask");

    let install = match package_manager.as_deref() {
        Some("uv") => "uv sync".to_string(),
        Some("poetry") => "poetry install".to_string(),
        Some("pipenv") => "pipenv install".to_string(),
        _ => {
            if pyproject.is_some() && requirements.is_none() {
                "pip install -e .".to_string()
            } else {
                "pip install -r requirements.txt".to_string()
            }
        }
    };

    let has_tests = root.join("tests").exists();

    if is_django {
        let mut evidence = vec![Some(if manage_py {
            "Detected manage.py".to_string()
        } else {
            "Detected Django dependency".to_string()
        })];
        if pyproject.is_some() {
            evidence.push(Some("Detected pyproject.toml".to_string()));
        }
        return Some(Recipe {
            name: "Django app".to_string(),
            kind: "django".to_string(),
            bootstrap: vec![install],
            build: vec![],
            test: vec!["python manage.py test".to_string()],
            start: Some("python manage.py runserver 0.0.0.0:8000".to_string()),
            port: Some(8000),
            readiness_path: "/".to_string(),
            evidence: dedupe(evidence),
        });
    }

    if is_fastapi {
        let app_module = if root.join("main.py").exists() {
            "main:app"
        } else if root.join("app.py").exists() {
            "app:app"
        } else {
            "main:app"
        };
        return Some(Recipe {
            name: "FastAPI app".to_string(),
            kind: "fastapi".to_string(),
            bootstrap: vec![install],
            build: vec![],
            test: if has_tests {
                vec!["pytest".to_string()]
            } else {
                vec![]
            },
            start: Some(format!("uvicorn {app_module} --host 0.0.0.0 --port 8000")),
            port: Some(8000),
            readiness_path: "/".to_string(),
            evidence: dedupe(vec![
                Some("Detected Python project".to_string()),
                Some("Detected FastAPI/Uvicorn dependency".to_string()),
            ]),
        });
    }

    if is_flask {
        let app_module = if root.join("app.py").exists() {
            "app.py"
        } else if root.join("main.py").exists() {
            "main.py"
        } else {
            "app.py"
        };
        return Some(Recipe {
            name: "Flask app".to_string(),
            kind: "flask".to_string(),
            bootstrap: vec![install],
            build: vec![],
            test: if has_tests {
                vec!["pytest".to_string()]
            } else {
                vec![]
            },
            start: Some(format!(
                "flask --app {app_module} run --host 0.0.0.0 --port 5000"
            )),
            port: Some(5000),
            readiness_path: "/".to_string(),
            evidence: dedupe(vec![
                Some("Detected Python project".to_string()),
                Some("Detected Flask dependency".to_string()),
            ]),
        });
    }

    Some(Recipe {
        name: "Python project".to_string(),
        kind: "python".to_string(),
        bootstrap: vec![install],
        build: vec![],
        test: if has_tests {
            vec!["pytest".to_string()]
        } else {
            vec!["python -m unittest discover".to_string()]
        },
        start: None,
        port: None,
        readiness_path: "/".to_string(),
        evidence: dedupe(vec![Some("Detected Python project".to_string())]),
    })
}

// ---------------------------------------------------------------------------
// Go / Rust / Java / Make / docker-compose
// ---------------------------------------------------------------------------

/// PARITY: `_SIMPLE_TOOLCHAINS` Go row (upstream lines 217-231).
fn detect_go_recipe(root: &Path) -> Option<Recipe> {
    if !root.join("go.mod").exists() {
        return None;
    }
    Some(Recipe {
        name: "Go project".to_string(),
        kind: "go".to_string(),
        bootstrap: vec![],
        build: vec!["go build ./...".to_string()],
        test: vec!["go test ./...".to_string()],
        start: if root.join("main.go").exists() {
            Some("go run .".to_string())
        } else {
            None
        },
        port: None,
        readiness_path: "/".to_string(),
        evidence: dedupe(vec![Some("Detected go.mod".to_string())]),
    })
}

/// PARITY: `_SIMPLE_TOOLCHAINS` Rust row (upstream lines 217-231).
fn detect_rust_recipe(root: &Path) -> Option<Recipe> {
    if !root.join("Cargo.toml").exists() {
        return None;
    }
    Some(Recipe {
        name: "Rust project".to_string(),
        kind: "rust".to_string(),
        bootstrap: vec![],
        build: vec!["cargo build".to_string()],
        test: vec!["cargo test".to_string()],
        start: if root.join("src").join("main.rs").exists() {
            Some("cargo run".to_string())
        } else {
            None
        },
        port: None,
        readiness_path: "/".to_string(),
        evidence: dedupe(vec![Some("Detected Cargo.toml".to_string())]),
    })
}

/// PARITY: `_detect_java_recipe` (upstream lines 234-243).
fn detect_java_recipe(root: &Path) -> Option<Recipe> {
    if root.join("pom.xml").exists() {
        return Some(Recipe {
            name: "Maven project".to_string(),
            kind: "maven".to_string(),
            bootstrap: vec![],
            build: vec!["mvn package".to_string()],
            test: vec!["mvn test".to_string()],
            start: None,
            port: None,
            readiness_path: "/".to_string(),
            evidence: dedupe(vec![Some("Detected pom.xml".to_string())]),
        });
    }
    if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
        let gradle = if root.join("gradlew").exists() {
            "./gradlew"
        } else {
            "gradle"
        };
        return Some(Recipe {
            name: "Gradle project".to_string(),
            kind: "gradle".to_string(),
            bootstrap: vec![],
            build: vec![format!("{gradle} build")],
            test: vec![format!("{gradle} test")],
            start: None,
            port: None,
            readiness_path: "/".to_string(),
            evidence: dedupe(vec![Some("Detected Gradle build file".to_string())]),
        });
    }
    None
}

/// PARITY: `_MAKE_TARGET_RE` (upstream line 415).
static MAKE_TARGET_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([A-Za-z0-9_.-]+):(?:\s|$)").expect("make target re"));

/// PARITY: make target scan (upstream lines 256-269).
fn parse_make_targets(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| MAKE_TARGET_RE.captures(line))
        .map(|caps| caps[1].to_string())
        .collect()
}

/// PARITY: `_detect_make_recipe` (upstream lines 256-269).
fn detect_make_recipe(root: &Path) -> Option<Recipe> {
    let makefile = read_text(root, "Makefile")?;
    let targets = parse_make_targets(&makefile);

    let pick = |names: &[&str]| -> Option<String> {
        names
            .iter()
            .find_map(|name| targets.iter().find(|t| t == name).cloned())
    };

    let install = pick(&["install", "setup", "bootstrap"]);
    let build = pick(&["build", "compile"]);
    let test = pick(&["test", "check"]);
    let run = pick(&["run", "start", "serve", "dev"]);

    let target_list = if targets.is_empty() {
        "(none)".to_string()
    } else {
        targets.join(", ")
    };
    let evidence = vec![
        Some("Detected Makefile".to_string()),
        Some(format!("Targets: {target_list}")),
    ];

    Some(Recipe {
        name: "Makefile-driven project".to_string(),
        kind: "make".to_string(),
        bootstrap: install
            .map(|i| vec![format!("make {i}")])
            .unwrap_or_default(),
        build: build.map(|b| vec![format!("make {b}")]).unwrap_or_default(),
        test: test.map(|t| vec![format!("make {t}")]).unwrap_or_default(),
        start: run.map(|r| format!("make {r}")),
        port: None,
        readiness_path: "/".to_string(),
        evidence: dedupe(evidence),
    })
}

/// PARITY: `_COMPOSE_FILES` (upstream lines 272).
const COMPOSE_FILES: [&str; 4] = [
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

/// PARITY: `_detect_compose_recipe` (upstream lines 275-280).
fn detect_compose_recipe(root: &Path) -> Option<Recipe> {
    let compose_file = COMPOSE_FILES
        .iter()
        .find(|f| root.join(f).exists())
        .to_owned()?;
    Some(Recipe {
        name: "docker-compose project".to_string(),
        kind: "compose".to_string(),
        bootstrap: vec![],
        build: vec!["docker compose build".to_string()],
        test: vec![],
        start: Some("docker compose up".to_string()),
        port: None,
        readiness_path: "/".to_string(),
        evidence: dedupe(vec![Some(format!("Detected {compose_file}"))]),
    })
}

// ---------------------------------------------------------------------------
// entry point
// ---------------------------------------------------------------------------

/// Detect a verification recipe for the project at `root`.
///
/// Detection order mirrors grok-cli's `inferFallbackRecipe`: package.json
/// wins, then Python, Go, Rust, Java, then Makefile / docker-compose
/// fallbacks. Returns `None` when nothing recognizable is found.
///
/// PARITY: `detect_recipe` (upstream lines 283-296).
pub fn detect_recipe(root: &Path) -> Option<Recipe> {
    if let Some(pkg) = read_package_json(root) {
        return Some(detect_node_recipe(root, &pkg));
    }
    detect_python_recipe(root)
        .or_else(|| detect_go_recipe(root))
        .or_else(|| detect_rust_recipe(root))
        .or_else(|| detect_java_recipe(root))
        .or_else(|| detect_make_recipe(root))
        .or_else(|| detect_compose_recipe(root))
}
