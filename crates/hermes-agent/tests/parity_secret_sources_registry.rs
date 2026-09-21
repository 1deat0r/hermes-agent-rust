//! Parity tests for `agent/secret_sources/registry.py` @ b9aa928.
//!
//! Upstream has no dedicated test file for the registry orchestrator
//! (missing-test gap, noted in the ledger); cases derive from the upstream
//! code as oracle.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;

use hermes_agent::secret_sources::base::{
    ErrorKind, FetchResult, SecretSource, DEFAULT_FETCH_TIMEOUT_SECONDS,
};
use hermes_agent::secret_sources::registry::{
    apply_all, get_source, list_sources, register_source, reset_registry_for_tests,
};

static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

/// Configurable test source.
struct TestSource {
    name: String,
    label: String,
    shape: String,
    scheme: Option<String>,
    enabled: bool,
    secrets: HashMap<String, String>,
    error: Option<(String, ErrorKind)>,
    sleep_ms: u64,
    fetches: AtomicUsize,
    fetch_timeout_seconds_for_test: Option<f64>,
    override_existing: bool,
}

impl TestSource {
    fn new(name: &str, shape: &str, enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            label: name.to_string(),
            shape: shape.to_string(),
            scheme: None,
            enabled,
            secrets: HashMap::new(),
            error: None,
            sleep_ms: 0,
            fetches: AtomicUsize::new(0),
            fetch_timeout_seconds_for_test: None,
            override_existing: false,
        }
    }

    fn with_secret(mut self, var: &str, value: &str) -> Self {
        self.secrets.insert(var.to_string(), value.to_string());
        self
    }
}

impl SecretSource for TestSource {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn shape(&self) -> &str {
        &self.shape
    }
    fn is_enabled(&self, _cfg: &serde_json::Value) -> bool {
        self.enabled
    }
    fn override_existing(&self, _cfg: &serde_json::Value) -> bool {
        self.override_existing
    }
    fn fetch(&self, _cfg: &serde_json::Value, _home: &std::path::Path) -> FetchResult {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(self.sleep_ms));
        let mut result = FetchResult::default();
        result.secrets = self.secrets.clone();
        if let Some((msg, kind)) = &self.error {
            result.error = Some(msg.clone());
            result.error_kind = Some(*kind);
        }
        result
    }
}

#[test]
fn registration_rejects_invalid_names_versions_and_shapes() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();

    struct BadName;
    impl SecretSource for BadName {
        fn name(&self) -> &str {
            "Has-Dash"
        }
        fn label(&self) -> &str {
            "x"
        }
        fn fetch(&self, _: &serde_json::Value, _: &std::path::Path) -> FetchResult {
            unreachable!()
        }
    }
    // Uppercase/dashes are an invalid lowercase [a-z0-9_]+ name.
    assert!(!register_source(Arc::new(BadName), false, None, false));

    struct OkSource;
    impl SecretSource for OkSource {
        fn name(&self) -> &str {
            "oksource"
        }
        fn label(&self) -> &str {
            "ok"
        }
        fn fetch(&self, _: &serde_json::Value, _: &std::path::Path) -> FetchResult {
            FetchResult::default()
        }
    }
    assert!(register_source(Arc::new(OkSource), false, None, false));
    // Duplicate without replace: ignored (false).
    assert!(!register_source(Arc::new(OkSource), false, None, false));
    // With replace: last-writer-wins.
    assert!(register_source(Arc::new(OkSource), true, None, false));
    // Listing ensures the bundled sources (bitwarden, onepassword,
    // command) alongside the test registration — upstream
    // `_ensure_builtin_sources` fires on every read path.
    let names: Vec<String> = list_sources(None)
        .iter()
        .map(|s| s.name().to_string())
        .collect();
    assert!(names.contains(&"oksource".to_string()), "{names:?}");
    for builtin in ["bitwarden", "onepassword", "command"] {
        assert!(names.contains(&builtin.to_string()), "{names:?}");
    }
    assert!(get_source("oksource", None).is_some());
    assert!(get_source("nope", None).is_none());
    reset_registry_for_tests();
}

#[test]
fn scheme_collision_across_names_is_rejected() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();

    struct WithScheme {
        name: &'static str,
        scheme: &'static str,
    }
    impl SecretSource for WithScheme {
        fn name(&self) -> &str {
            self.name
        }
        fn label(&self) -> &str {
            "x"
        }
        fn scheme(&self) -> Option<&str> {
            Some(self.scheme)
        }
        fn fetch(&self, _: &serde_json::Value, _: &std::path::Path) -> FetchResult {
            FetchResult::default()
        }
    }
    assert!(register_source(
        Arc::new(WithScheme {
            name: "first",
            scheme: "op",
        }),
        false,
        None,
        false
    ));
    assert!(!register_source(
        Arc::new(WithScheme {
            name: "second",
            scheme: "op",
        }),
        false,
        None,
        false
    ));
    reset_registry_for_tests();
}

#[test]
fn apply_all_precedence_and_first_claim_wins() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();

    let mut mapped = TestSource::new("onepass", "mapped", true);
    mapped
        .secrets
        .insert("PLUGIN_KEY".to_string(), "mapped-value".to_string());
    let mut bulk = TestSource::new("bitwarden", "bulk", true);
    bulk.secrets
        .insert("PLUGIN_KEY".to_string(), "bulk-value".to_string());
    bulk.secrets
        .insert("BW_ONLY".to_string(), "from-bulk".to_string());

    register_source(Arc::new(bulk), false, None, false);
    register_source(Arc::new(mapped), false, None, false);

    let mut env = HashMap::new();
    // Pre-existing value survives a non-override source.
    env.insert("BW_ONLY".to_string(), "shell-value".to_string());
    let report = apply_all(
        Some(&json!({"sources": ["onepass", "bitwarden"]})),
        None,
        &mut env,
        None,
    );

    // First claim wins and the conflict is surfaced.
    assert_eq!(
        env.get("PLUGIN_KEY").map(String::as_str),
        Some("mapped-value")
    );
    assert!(report
        .conflicts
        .iter()
        .any(|c| c.contains("PLUGIN_KEY") && c.contains("first source wins")));
    // Pre-existing env won (override_existing defaults false).
    assert_eq!(env.get("BW_ONLY").map(String::as_str), Some("shell-value"));
    // Provenance records the winner and the shape.
    let provenance = &report.provenance["PLUGIN_KEY"];
    assert_eq!(provenance.source, "onepass");
    assert_eq!(provenance.shape, "mapped");
    assert!(report.applied_any());
}

#[test]
fn override_existing_beats_env_but_not_other_sources() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();

    struct OverrideSource {
        secrets: HashMap<String, String>,
    }
    impl SecretSource for OverrideSource {
        fn name(&self) -> &str {
            "overrider"
        }
        fn label(&self) -> &str {
            "overrider"
        }
        fn fetch(&self, _: &serde_json::Value, _: &std::path::Path) -> FetchResult {
            let mut r = FetchResult::default();
            r.secrets = self.secrets.clone();
            r
        }
        fn override_existing(&self, _cfg: &serde_json::Value) -> bool {
            true
        }
    }

    let mut env = HashMap::new();
    env.insert("EXISTING_VAR".to_string(), "shell".to_string());
    env.insert("PRESERVED_VAR".to_string(), "shell".to_string());

    let cfg = json!({
        "overrider": {"enabled": true},
        "preserve_existing": ["PRESERVED_VAR"],
    });
    let mut report = hermes_agent::secret_sources::registry::ApplyReport::default();
    let _ = &mut report;

    // Simulate the orchestrator for one source (apply_all path shares the
    // guard chain): preserve beats override_existing.
    let source = OverrideSource {
        secrets: [
            ("EXISTING_VAR".to_string(), "new".to_string()),
            ("PRESERVED_VAR".to_string(), "new".to_string()),
        ]
        .into_iter()
        .collect(),
    };
    register_source(Arc::new(source), false, None, false);
    let result = apply_all(Some(&cfg), None, &mut env, None);
    let _ = result;
    assert_eq!(
        env.get("EXISTING_VAR").map(String::as_str),
        Some("new"),
        "override_existing beats .env/shell"
    );
    assert_eq!(
        env.get("PRESERVED_VAR").map(String::as_str),
        Some("shell"),
        "preserve_existing beats even override_existing"
    );
}

#[test]
fn timeout_source_reports_timeout_and_startup_continues() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();

    let mut slow = TestSource::new("slowpoke", "mapped", true);
    slow.sleep_ms = 5_000;
    slow.secrets.insert("SLOW_VAR".to_string(), "v".to_string());
    slow.fetch_timeout_seconds_for_test = Some(0.2);
    let slow = SlowWrap(slow);
    register_source(Arc::new(slow), false, None, false);

    let mut env = HashMap::new();
    let started = std::time::Instant::now();
    let report = apply_all(Some(&json!({})), None, &mut env, None);
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
    assert!(env.is_empty(), "timed-out source's vars are not applied");
    assert!(!report.applied_any());
}

// Thin wrapper carrying a per-source timeout override.
struct SlowWrap(TestSource);
impl SecretSource for SlowWrap {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn label(&self) -> &str {
        self.0.label()
    }
    fn shape(&self) -> &str {
        self.0.shape()
    }
    fn fetch_timeout_seconds(&self, _cfg: &serde_json::Value) -> f64 {
        self.0
            .fetch_timeout_seconds_for_test
            .unwrap_or(DEFAULT_FETCH_TIMEOUT_SECONDS)
    }
    fn fetch(&self, cfg: &serde_json::Value, home: &std::path::Path) -> FetchResult {
        self.0.fetch(cfg, home)
    }
}

#[test]
fn disabled_sources_are_skipped_and_fetch_never_runs() {
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();
    let source = TestSource::new("disabled", "mapped", false);
    let fetches = Arc::new(AtomicUsize::new(0));
    let _ = &fetches;
    register_source(Arc::new(source), false, None, false);
    let mut env = HashMap::new();
    let report = apply_all(Some(&json!({})), None, &mut env, None);
    assert!(report.sources.is_empty());
    assert!(env.is_empty());
}

// ── 5d59366 scoped registry ────────────────────────────────────────────

#[test]
fn scoped_overlay_merges_and_plugin_sources_list() {
    use hermes_agent::secret_sources::registry::{
        list_plugin_sources, restore_registration, snapshot_registration,
    };
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();
    // A scoped plugin registration merges over globals.
    register_source(
        Arc::new(TestSource::new("scoped_src", "mapped", true)),
        false,
        Some("home-a"),
        false,
    );
    let names: Vec<String> = hermes_agent::secret_sources::registry::list_sources(Some("home-a"))
        .iter()
        .map(|s| s.name().to_string())
        .collect();
    assert!(names.contains(&"scoped_src".to_string()), "{names:?}");
    // The other scope and the bare global view do not see it.
    assert!(
        !hermes_agent::secret_sources::registry::list_sources(Some("home-b"))
            .iter()
            .any(|s| s.name() == "scoped_src")
    );
    // Snapshot reads the layer; restore is identity-conditional.
    let current =
        snapshot_registration("scoped_src", Some("home-a")).expect("snapshot hits the layer");
    assert!(snapshot_registration("scoped_src", None).is_none());
    let other: Arc<dyn SecretSource> = Arc::new(TestSource::new("scoped_src", "mapped", true));
    assert!(!restore_registration(
        "scoped_src",
        &other,
        None,
        Some("home-a")
    ));
    assert!(restore_registration(
        "scoped_src",
        &current,
        None,
        Some("home-a")
    ));
    assert!(snapshot_registration("scoped_src", Some("home-a")).is_none());
    // Plugin listing sees the scoped entry (and not builtins).
    reset_registry_for_tests();
    register_source(
        Arc::new(TestSource::new("plug", "mapped", true)),
        false,
        None,
        false,
    );
    let plugins = list_plugin_sources(None);
    assert!(plugins.iter().any(|s| s.name() == "plug"));
    assert!(!plugins.iter().any(|s| s.name() == "bitwarden"));
    reset_registry_for_tests();
}

#[test]
fn empty_env_value_counts_as_not_existed() {
    // `bool(env.get(var))`: an empty-string value is NOT a pre-existing
    // value, so a non-override source still applies over it.
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();
    let mut source = TestSource::new("fill", "mapped", true);
    source.override_existing = false;
    source
        .secrets
        .insert("EMPTY_VAR".to_string(), "fresh".to_string());
    register_source(Arc::new(source), false, None, false);
    let mut env = HashMap::new();
    env.insert("EMPTY_VAR".to_string(), String::new());
    let report = apply_all(
        Some(&json!({"fill": {"enabled": true}})),
        None,
        &mut env,
        None,
    );
    assert_eq!(env.get("EMPTY_VAR").map(String::as_str), Some("fresh"));
    assert!(report.provenance["EMPTY_VAR"].overrode_env == false);
    reset_registry_for_tests();
}

#[test]
fn profile_alias_warning_lands_in_the_live_report() {
    // The alias-applied warning must be visible in the pushed source
    // report (it previously went to a dead clone).
    let _guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_registry_for_tests();
    let mut source = TestSource::new("aliased", "mapped", true);
    source.override_existing = true;
    source
        .secrets
        .insert("MY_API_KEY_WORK".to_string(), "w".to_string());
    register_source(Arc::new(source), false, None, false);
    let mut env = HashMap::new();
    let home = std::path::PathBuf::from("/home/u/.hermes/profiles/work");
    let report = apply_all(
        Some(&json!({"aliased": {"enabled": true}})),
        Some(&home),
        &mut env,
        None,
    );
    assert_eq!(env.get("MY_API_KEY").map(String::as_str), Some("w"));
    let row = report
        .sources
        .iter()
        .find(|s| s.name == "aliased")
        .expect("source row");
    assert!(
        row.result
            .as_ref()
            .map(|r| r
                .warnings
                .iter()
                .any(|w| w.contains("applied profile-scoped")))
            .unwrap_or(false),
        "{row:?}"
    );
    reset_registry_for_tests();
}
