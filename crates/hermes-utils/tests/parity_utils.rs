//! Parity oracles for `hermes-utils` vs upstream `utils.py` @ 5d59366.
//!
//! Sources: `upstream/golden_utils.json` (generated fixtures) + the root
//! utils oracle files — `test_utils_truthy_values.py`,
//! `test_base_url_hostname.py`, `test_fast_safe_load.py`,
//! `test_atomic_write_text_metadata.py`, `test_atomic_replace_symlinks.py`,
//! `test_atomic_json_writers_unified.py` (utils-owned halves),
//! `test_utils_atomic_roundtrip_yaml_save.py`,
//! `test_credential_file_permissions.py`,
//! `test_yaml_indent_consistency.py`.
//!
//! Upstream monkeypatches (`os.replace`, `_IS_WINDOWS`, `_preserve_file_owner`,
//! `os.chown`, retry delays, `builtins.open`) map to the documented test seams
//! in `hermes_utils::atomic`. Skipped-with-note upstream cases:
//! - `@windows_only` real-handle tests (no Windows lane in this workspace;
//!   the cross-platform retry/fallback state machine runs via seams).
//! - Lone-surrogate JSON round-trip (`os.fsdecode` artifacts) — Rust `String`
//!   cannot represent lone surrogates.
//! - `test_stale_utils_module_import` — Python module-cache staleness has no
//!   Rust analog (static linkage).
//! - The PyYAML-default-0-indent baseline in the indent suite (PyYAML-
//!   specific; our emitter is exercised by the IndentDumper-side asserts).
//! - str-vs-PathLike dual inputs (Rust type system: `&Path` only).
//! - Google-chat / gateway / cron / shell-hook consumer rows (other ledger
//!   rows); the shared contract they lean on (failed replace ⇒ old bytes +
//!   no temp) is pinned here at the utils level via the replace-hook seam.
//!
//! Tier: `unit`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

use hermes_utils::{
    atomic_json_write, atomic_replace, atomic_roundtrip_yaml_save, atomic_write_bytes,
    atomic_write_text, atomic_yaml_write, base_url_host_matches, base_url_hostname,
    base_url_origin, env_bool, env_float, env_int, fast_safe_load, file_signature,
    force_windows_contended_for_test, fsync_directory, is_truthy, normalize_proxy_url,
    read_json_or_empty, reset_owner_seams_for_test, reset_replace_hook_for_test,
    reset_replace_retry_delays_for_test, set_owner_seams_for_test, set_replace_hook_for_test,
    set_replace_retry_delays_for_test, warn_if_credential_file_broadly_readable, TruthyValue,
    REPLACE_RETRY_ATTEMPTS,
};
use serde_json::Value;

// ── golden fixtures (pre-retarget, still valid at 5d59366) ────────────────

fn load_golden() -> Value {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("upstream/golden_utils.json");
    let text = std::fs::read_to_string(&path).expect("golden fixture missing");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn is_truthy_value_matches_upstream_golden() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let g = load_golden();
    let tc = &g["is_truthy_value"];
    let expect = |key: &str, value: TruthyValue, default: bool| {
        let got = is_truthy(&value, default);
        let want = tc[key].as_bool().unwrap();
        assert_eq!(got, want, "key {}: got {} want {}", key, got, want);
    };
    for (key_str, v) in [
        ("True", TruthyValue::Bool(true)),
        ("False", TruthyValue::Bool(false)),
        ("'1'", TruthyValue::Str("1")),
        ("'true'", TruthyValue::Str("true")),
        ("'TRUE'", TruthyValue::Str("TRUE")),
        ("'Yes'", TruthyValue::Str("Yes")),
        ("'on'", TruthyValue::Str("on")),
        ("'0'", TruthyValue::Str("0")),
        ("'false'", TruthyValue::Str("false")),
        ("'no'", TruthyValue::Str("no")),
        ("'off'", TruthyValue::Str("off")),
        ("'yes sir'", TruthyValue::Str("yes sir")),
        ("''", TruthyValue::Str("")),
        ("'  on  '", TruthyValue::Str("  on  ")),
    ] {
        expect(key_str, v, false);
    }
}

#[test]
fn model_forces_max_completion_tokens_matches_upstream_golden() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let g = load_golden();
    let mc = &g["model_forces_max_completion_tokens"];
    for (model, want) in mc.as_object().unwrap() {
        let got = hermes_utils::model_forces_max_completion_tokens(model);
        let want = want.as_bool().unwrap();
        assert_eq!(got, want, "model {:?}: got {} want {}", model, got, want);
    }
}

#[test]
fn base_url_hostname_matches_upstream_golden() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let g = load_golden();
    let hh = &g["base_url_hostname"];
    for (url, want) in hh.as_object().unwrap() {
        let got = base_url_hostname(url);
        let want = want.as_str().unwrap();
        assert_eq!(got, want, "url {:?}: got {:?} want {:?}", url, got, want);
    }
}

#[test]
fn base_url_host_matches_upstream_golden() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let g = load_golden();
    let hm = &g["base_url_host_matches"];
    for (record, want) in hm.as_object().unwrap() {
        let (url, domain) = record.split_once(" || ").unwrap();
        let got = base_url_host_matches(url, domain);
        let want = want.as_bool().unwrap();
        assert_eq!(got, want, "record {}", record);
    }
}

#[test]
fn normalize_proxy_url_matches_upstream_golden() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let g = load_golden();
    let np = &g["normalize_proxy_url"];
    for (rec, want) in np.as_object().unwrap() {
        let input: Option<&str> = match rec.as_str() {
            "None" => None,
            "'  '" => Some("  "),
            other => Some(other.trim_matches('\'')),
        };
        let got = normalize_proxy_url(input);
        match want {
            Value::Null => assert!(
                got.is_none(),
                "record {}: expected None, got {:?}",
                rec,
                got
            ),
            Value::String(s) => assert_eq!(got.as_deref(), Some(s.as_str()), "record {}", rec),
            other => panic!("unexpected golden shape for {}: {}", rec, other),
        }
    }
}

// ── test_utils_truthy_values.py ───────────────────────────────────────────

static ENV_LOCK: Mutex<()> = Mutex::new(());
/// Serializes ALL tests in this binary: the replace hook, owner seams,
/// windows flag, retry delays, log sink and env vars are process-global
/// (upstream monkeypatches + caplog share the same hazard but pytest's
/// default is also parallel — upstream suites isolate via fixtures).
static SEAM_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn oracle_is_truthy_value_accepts_common_truthy_strings() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert!(is_truthy(&TruthyValue::Str("true"), false));
    assert!(is_truthy(&TruthyValue::Str(" YES "), false));
    assert!(is_truthy(&TruthyValue::Str("on"), false));
    assert!(is_truthy(&TruthyValue::Str("1"), false));
}

#[test]
fn oracle_is_truthy_value_respects_default_for_none() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert!(is_truthy(&TruthyValue::Missing, true));
    assert!(!is_truthy(&TruthyValue::Missing, false));
}

#[test]
fn oracle_is_truthy_value_rejects_falsey_strings() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert!(!is_truthy(&TruthyValue::Str("false"), false));
    assert!(!is_truthy(&TruthyValue::Str("0"), false));
    assert!(!is_truthy(&TruthyValue::Str("off"), false));
}

#[test]
fn oracle_env_var_enabled_uses_shared_truthy_rules() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _g = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("HERMES_TEST_BOOL", "YeS") };
    assert!(hermes_utils::env_var_enabled("HERMES_TEST_BOOL", ""));
    unsafe { std::env::set_var("HERMES_TEST_BOOL", "no") };
    assert!(!hermes_utils::env_var_enabled("HERMES_TEST_BOOL", ""));
    unsafe { std::env::remove_var("HERMES_TEST_BOOL") };
}

#[test]
fn oracle_env_int_env_float_fallbacks() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // utils.py `_env_number` / env_int / env_float: strip + cast, fallback
    // on empty/absent/garbage.
    let _g = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("HERMES_TEST_INT", "42") };
    unsafe { std::env::set_var("HERMES_TEST_BAD", "x") };
    unsafe { std::env::set_var("HERMES_TEST_F", "2.5") };
    assert_eq!(env_int("HERMES_TEST_INT", 7), 42);
    assert_eq!(env_int("HERMES_TEST_BAD", 7), 7);
    assert_eq!(env_int("HERMES_UNSET_FOR_TEST", 7), 7);
    assert_eq!(env_float("HERMES_TEST_F", 1.0), 2.5);
    assert_eq!(env_float("HERMES_TEST_BAD", 1.0), 1.0);
    // env_bool: oracle-unpinned quirk in upstream (default is dead for
    // missing vars); the port honors the default — documented divergence
    // (PLAN.md). Present-falsy strings still coerce to false.
    assert!(env_bool("HERMES_TEST_UNSET_T", true));
    unsafe { std::env::set_var("HERMES_TEST_BOOL2", "no") };
    assert!(!env_bool("HERMES_TEST_BOOL2", true));
    for k in [
        "HERMES_TEST_INT",
        "HERMES_TEST_BAD",
        "HERMES_TEST_F",
        "HERMES_TEST_BOOL2",
    ] {
        unsafe { std::env::remove_var(k) };
    }
}

// ── test_base_url_hostname.py ─────────────────────────────────────────────

#[test]
fn oracle_hostname_empty_and_plain_host() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(base_url_hostname(""), "");
    assert_eq!(base_url_hostname("api.openai.com"), "api.openai.com");
    assert_eq!(base_url_hostname("api.openai.com/v1"), "api.openai.com");
    assert_eq!(
        base_url_hostname("https://api.openai.com./v1"),
        "api.openai.com",
        "trailing dot stripped"
    );
    assert_eq!(
        base_url_hostname("https://api.openai.com.example/v1"),
        "api.openai.com.example"
    );
    assert_eq!(
        base_url_hostname("https://api.openai.com:443/v1"),
        "api.openai.com"
    );
}

#[test]
fn oracle_host_matches_exact_subdomain_and_negatives() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Exact + subdomain (TestBaseUrlHostMatchesExact).
    assert!(base_url_host_matches(
        "https://openrouter.ai/api/v1",
        "openrouter.ai"
    ));
    assert!(base_url_host_matches(
        "https://api.moonshot.ai/v1",
        "moonshot.ai"
    ));
    assert!(base_url_host_matches(
        "https://api.kimi.com/v1",
        "api.kimi.com"
    ));
    // Path-segment / suffix / prefix collisions (TestBaseUrlHostMatchesNegatives).
    assert!(!base_url_host_matches(
        "https://evil.test/moonshot.ai/v1",
        "moonshot.ai"
    ));
    assert!(!base_url_host_matches(
        "https://proxy.example.test/openrouter.ai/v1",
        "openrouter.ai"
    ));
    assert!(!base_url_host_matches(
        "https://moonshot.ai.evil/v1",
        "moonshot.ai"
    ));
    assert!(!base_url_host_matches(
        "https://openrouter.ai.example/v1",
        "openrouter.ai"
    ));
    assert!(!base_url_host_matches(
        "https://fake-openrouter.ai/v1",
        "openrouter.ai"
    ));
    // Empty base URL → false (TestBaseUrlHostMatchesEdgeCases).
    assert!(!base_url_host_matches("", "openrouter.ai"));
    // Ollama GHSA vectors (TestOllamaUrlHostCheck).
    assert!(!base_url_host_matches(
        "http://127.0.0.1:9000/ollama.com/v1",
        "ollama.com"
    ));
    assert!(!base_url_host_matches(
        "http://ollama.com.attacker.test:9000/v1",
        "ollama.com"
    ));
    assert!(base_url_host_matches(
        "https://ollama.com/api/generate",
        "ollama.com"
    ));
}

#[test]
fn oracle_base_url_origin_effective_ports() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // utils.py `base_url_origin` — probe-pinned against the oracle:
    // origin = (scheme, hostname, effective port); port defaults per-scheme;
    // unusable input → ("", "", 0).
    assert_eq!(
        base_url_origin("api.openai.com"),
        (String::new(), "api.openai.com".to_string(), 0),
        "scheme-less URL: empty scheme, host kept, default port 0"
    );
    assert_eq!(
        base_url_origin("https://h"),
        ("https".to_string(), "h".to_string(), 443)
    );
    assert_eq!(
        base_url_origin("https://h:443"),
        ("https".to_string(), "h".to_string(), 443),
        "explicit default port equals implicit"
    );
    assert_eq!(
        base_url_origin("http://h:80"),
        ("http".to_string(), "h".to_string(), 80)
    );
    assert_eq!(
        base_url_origin("https://h:99999"),
        (String::new(), String::new(), 0),
        "out-of-range port → unusable"
    );
    assert_eq!(base_url_origin(""), (String::new(), String::new(), 0));
    assert_eq!(
        base_url_origin("h:8080"),
        (String::new(), "h".to_string(), 8080),
        "bare host with port keeps port, scheme stays empty"
    );
    assert_eq!(base_url_origin("/v1"), (String::new(), String::new(), 0));
    assert_eq!(
        base_url_origin("https://API.OpenAI.com./v1"),
        ("https".to_string(), "api.openai.com".to_string(), 443)
    );
}

// ── test_fast_safe_load.py ────────────────────────────────────────────────

#[test]
fn oracle_fast_safe_load_docs_and_empty() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for doc in [
        "",
        "a: 1\nb: two\nc: 3.5\n",
        "list: [1, 2, 3]\nnested:\n  k: v\n  flag: true\n  empty: null\n",
        "name: skill-x\nmetadata:\n  hermes:\n    tags: [alpha, beta]\n    category: devops\n",
        "- one\n- two\n- three\n",
        "scalar string",
    ] {
        let v = fast_safe_load(doc).expect("parses like safe_load");
        let _ = v;
    }
    // Empty document: upstream returns None (Python) — Rust Value::Null is
    // the None analog (callers treat both as "no document").
    assert_eq!(
        fast_safe_load("").unwrap(),
        serde_yaml::Value::Null,
        "empty → None/Null"
    );
    assert_eq!(fast_safe_load("a: 1\n").unwrap()["a"].as_i64(), Some(1));
    assert_eq!(
        fast_safe_load("- one\n- two\n").unwrap()[1].as_str(),
        Some("two")
    );
}

#[test]
fn oracle_fast_safe_load_rejects_python_object_tags() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // `test_rejects_arbitrary_python_objects_like_safe_load` — the tag is
    // accepted by unsafe loaders; safe loaders raise.
    let dangerous = "!!python/object/apply:os.system ['echo pwned']\n";
    assert!(
        fast_safe_load(dangerous).is_err(),
        "fast_safe_load must reject python/object tags like safe_load"
    );
    // Standard YAML tags stay legal.
    assert!(fast_safe_load("a: !!str 1\n").is_ok());
}

// ── log capture for the credential-warning oracle ─────────────────────────

struct CaptureLogger;

static LOG_SINK: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LOGGER_INIT: Once = Once::new();

fn install_capture_logger() {
    LOGGER_INIT.call_once(|| {
        let _ = log::set_logger(&CaptureLogger);
        log::set_max_level(log::LevelFilter::Trace);
    });
    LOG_SINK.lock().unwrap().clear();
}

impl log::Log for CaptureLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        if record.level() >= log::Level::Warn {
            LOG_SINK.lock().unwrap().push(format!("{}", record.args()));
        }
    }
    fn flush(&self) {}
}

fn logged_warnings() -> Vec<String> {
    LOG_SINK.lock().unwrap().clone()
}

// ── test_credential_file_permissions.py ───────────────────────────────────

#[cfg(unix)]
#[test]
fn oracle_credential_file_permission_warnings() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::PermissionsExt;
    install_capture_logger();
    let td = tempfile::TempDir::new().unwrap();

    // test_warns_on_world_readable — message shape: label + basename +
    // mode + chmod-600 remediation + full path.
    let f = td.path().join("slack_tokens.json");
    std::fs::write(&f, "{}").unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(warn_if_credential_file_broadly_readable(
        &f,
        Some("[Slack]")
    ));
    let warnings = logged_warnings();
    let msg = warnings.last().expect("warning emitted");
    assert!(msg.contains("group/world-readable"), "{msg}");
    assert!(msg.contains("chmod 600"), "{msg}");
    assert!(msg.contains("[Slack]"), "{msg}");
    assert!(msg.contains("slack_tokens.json"), "{msg}");
    assert!(msg.contains(&f.display().to_string()), "{msg}");

    // test_warns_on_group_readable.
    let g = td.path().join("tokens.json");
    std::fs::write(&g, "{}").unwrap();
    std::fs::set_permissions(&g, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert!(warn_if_credential_file_broadly_readable(&g, None));

    // test_silent_on_0600.
    std::fs::set_permissions(&g, std::fs::Permissions::from_mode(0o600)).unwrap();
    LOG_SINK.lock().unwrap().clear();
    assert!(!warn_if_credential_file_broadly_readable(&g, None));
    assert!(logged_warnings().is_empty());

    // test_silent_on_missing_file.
    assert!(!warn_if_credential_file_broadly_readable(
        &td.path().join("nope.json"),
        None
    ));
    assert!(logged_warnings().is_empty());
}

// ── test_atomic_write_text_metadata.py ────────────────────────────────────

#[cfg(unix)]
#[test]
fn oracle_atomic_write_text_preserve_mode_class() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::PermissionsExt;
    let td = tempfile::TempDir::new().unwrap();

    // test_existing_mode_survives_the_rewrite (0640 must not tighten to 0600).
    let target = td.path().join("config.yaml");
    std::fs::write(&target, "old: true\n").unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
    atomic_write_text(&target, "new: true\n", true, None, None, false).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new: true\n");
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o640
    );

    // test_default_still_leaves_mkstemp_mode (no opt-in → existing 0644
    // becomes 0600 — upstream "existing default" semantics).
    let notes = td.path().join("notes.md");
    std::fs::write(&notes, "old\n").unwrap();
    std::fs::set_permissions(&notes, std::fs::Permissions::from_mode(0o644)).unwrap();
    atomic_write_text(&notes, "new\n", false, None, None, false).unwrap();
    assert_eq!(
        std::fs::metadata(&notes).unwrap().permissions().mode() & 0o777,
        0o600
    );

    // test_create_mode_applies_when_target_is_new.
    let soul = td.path().join("SOUL.md");
    assert!(!soul.exists());
    atomic_write_text(&soul, "# Persona\n", true, Some(0o644), None, false).unwrap();
    assert_eq!(
        std::fs::metadata(&soul).unwrap().permissions().mode() & 0o777,
        0o644
    );

    // test_existing_mode_beats_create_mode.
    let ex = td.path().join("existing.md");
    std::fs::write(&ex, "old\n").unwrap();
    std::fs::set_permissions(&ex, std::fs::Permissions::from_mode(0o600)).unwrap();
    atomic_write_text(&ex, "new\n", true, Some(0o644), None, false).unwrap();
    assert_eq!(
        std::fs::metadata(&ex).unwrap().permissions().mode() & 0o777,
        0o600
    );

    // test_create_mode_never_rewrites_an_existing_file.
    let keep = td.path().join("keep.md");
    std::fs::write(&keep, "old\n").unwrap();
    std::fs::set_permissions(&keep, std::fs::Permissions::from_mode(0o640)).unwrap();
    atomic_write_text(&keep, "new\n", false, Some(0o644), None, false).unwrap();
    assert_eq!(
        std::fs::metadata(&keep).unwrap().permissions().mode() & 0o777,
        0o600
    );

    // New file, no args: follows the process umask (oracle probe: with
    // umask 022 a fresh file lands 0644, not mkstemp's 0600).
    unsafe {
        let old = libc::umask(0o022);
        let fresh = td.path().join("fresh.md");
        atomic_write_text(&fresh, "hi\n", false, None, None, false).unwrap();
        assert_eq!(
            std::fs::metadata(&fresh).unwrap().permissions().mode() & 0o777,
            0o644,
            "new non-secret file follows umask"
        );
        libc::umask(old);
    }
}

#[cfg(unix)]
#[test]
fn oracle_mode_is_applied_before_the_replace_via_replace_hook() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_mode_is_applied_before_the_replace — the spying replace sees the
    // TEMP file already carrying 0640 (fchmod happened pre-replace).
    use std::os::unix::fs::PermissionsExt;
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("config.yaml");
    std::fs::write(&target, "old\n").unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();

    let seen: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
    let spy = seen.clone();
    set_replace_hook_for_test(Some(Box::new(
        move |tmp: &Path, dst: &Path| -> std::io::Result<PathBuf> {
            let mode = std::fs::metadata(tmp).unwrap().permissions().mode() & 0o777;
            spy.lock().unwrap().push(mode);
            std::fs::rename(tmp, dst)?;
            Ok(dst.to_path_buf())
        },
    )));
    atomic_write_text(&target, "new\n", true, None, None, false).unwrap();
    reset_replace_hook_for_test();
    assert_eq!(*seen.lock().unwrap(), vec![0o640]);
}

#[cfg(unix)]
#[test]
fn oracle_no_owner_calls_without_opt_in() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_no_owner_calls_without_opt_in — preserve flag off ⇒ the owner
    // reader is never consulted and chown never runs (seam records calls).
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("mem.md");
    std::fs::write(&target, "old\n").unwrap();
    let chowns: Arc<Mutex<Vec<(PathBuf, u32, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    set_owner_seams_for_test(
        Some(|_p: &Path| Some((123, 456))),
        Some(Box::new({
            let rec = chowns.clone();
            move |p: &Path, uid, gid| {
                rec.lock().unwrap().push((p.to_path_buf(), uid, gid));
            }
        })),
    );
    atomic_write_text(&target, "new\n", false, None, None, false).unwrap();
    reset_owner_seams_for_test();
    assert!(chowns.lock().unwrap().is_empty(), "no chown without opt-in");
}

#[cfg(unix)]
#[test]
fn oracle_owner_is_restored_on_the_real_symlink_target() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_owner_is_restored_on_the_real_symlink_target — forced uid/gid
    // (the upstream monkeypatch) lands on the REAL file through the link.
    use std::os::unix::fs::symlink;
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("zshrc");
    std::fs::write(&real, "export A=1\n").unwrap();
    let link = td.path().join(".zshrc");
    symlink(&real, &link).unwrap();

    let chowns: Arc<Mutex<Vec<(PathBuf, u32, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    set_owner_seams_for_test(
        Some(|_p: &Path| Some((123, 456))),
        Some(Box::new({
            let rec = chowns.clone();
            move |p: &Path, uid, gid| {
                rec.lock().unwrap().push((p.to_path_buf(), uid, gid));
            }
        })),
    );
    atomic_write_text(&link, "export B=2\n", true, None, None, false).unwrap();
    reset_owner_seams_for_test();

    assert_eq!(*chowns.lock().unwrap(), vec![(real.clone(), 123, 456)]);
    assert!(link.is_symlink());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "export B=2\n");
}

#[cfg(unix)]
#[test]
fn oracle_atomic_yaml_write_create_mode() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::PermissionsExt;
    let td = tempfile::TempDir::new().unwrap();
    // New file lands 0644 (create_mode), not 0600.
    let target = td.path().join("distribution.yaml");
    assert!(!target.exists());
    atomic_yaml_write(
        &target,
        &serde_json::json!({"name": "t"}),
        false,
        None,
        Some(0o644),
    )
    .unwrap();
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o644
    );
    // Existing mode beats create_mode.
    let existing = td.path().join("existing.yaml");
    std::fs::write(&existing, "name: old\n").unwrap();
    std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o600)).unwrap();
    atomic_yaml_write(
        &existing,
        &serde_json::json!({"name": "new"}),
        false,
        None,
        Some(0o644),
    )
    .unwrap();
    assert_eq!(
        std::fs::metadata(&existing).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

// ── test_atomic_replace_symlinks.py ───────────────────────────────────────

fn write_tmp(dir: &Path, content: &str) -> PathBuf {
    let tmp = dir.join(".src.tmp");
    std::fs::write(&tmp, content).unwrap();
    tmp
}

#[cfg(unix)]
#[test]
fn oracle_atomic_replace_preserves_symlink() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::symlink;
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("real.yaml");
    std::fs::write(&real, "original\n").unwrap();
    let link = td.path().join("link.yaml");
    symlink(&real, &link).unwrap();

    let tmp = write_tmp(td.path(), "updated\n");
    let returned = atomic_replace(&tmp, &link).expect("replace");
    assert!(link.is_symlink(), "symlink must not be replaced");
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "updated\n");
    assert_eq!(returned, real);
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "updated\n");
}

#[test]
fn oracle_atomic_replace_regular_file() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("plain.yaml");
    std::fs::write(&target, "old\n").unwrap();
    let tmp = write_tmp(td.path(), "fresh\n");
    let returned = atomic_replace(&tmp, &target).expect("replace");
    assert_eq!(returned, target);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "fresh\n");
    assert!(!target.is_symlink());
}

#[cfg(unix)]
#[test]
fn oracle_atomic_replace_broken_symlink_creates_target() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::symlink;
    // test_atomic_replace_broken_symlink_creates_target — resolve through
    // the dangling link; the real target is created, the link survives.
    let td = tempfile::TempDir::new().unwrap();
    let missing = td.path().join("does_not_exist_yet.yaml");
    let link = td.path().join("link.yaml");
    symlink(&missing, &link).unwrap();
    assert!(link.is_symlink());
    assert!(!missing.exists());

    let tmp = write_tmp(td.path(), "created-through-link\n");
    atomic_replace(&tmp, &link).expect("replace");

    assert!(link.is_symlink(), "symlink must be preserved");
    assert!(missing.exists(), "real target should now exist");
    assert_eq!(
        std::fs::read_to_string(&missing).unwrap(),
        "created-through-link\n"
    );
}

#[cfg(unix)]
#[test]
fn oracle_atomic_replace_copy_fallback_preserves_symlink() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_atomic_replace_copy_fallback_preserves_symlink — EXDEV forces
    // the copy path; the symlink and the temp cleanup still hold.
    use std::os::unix::fs::symlink;
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("real.yaml");
    std::fs::write(&real, "old\n").unwrap();
    let link = td.path().join("link.yaml");
    symlink(&real, &link).unwrap();
    let tmp = write_tmp(td.path(), "new\n");

    set_replace_hook_for_test(Some(Box::new(
        |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> {
            Err(std::io::Error::from_raw_os_error(18))
        },
    )));
    let returned = atomic_replace(&tmp, &link).expect("replace");
    reset_replace_hook_for_test();

    assert_eq!(returned, real);
    assert!(link.is_symlink());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "new\n");
    assert!(!tmp.exists(), "temp unlinked after copy");
}

#[cfg(unix)]
#[test]
fn oracle_atomic_replace_real_cross_device() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_atomic_replace_real_cross_device — /dev/shm as the other
    // filesystem (skips when unavailable or same device, like upstream).
    use std::os::unix::fs::symlink;
    let shm = Path::new("/dev/shm");
    if !shm.join(".").exists() || !is_writable(shm) {
        return; // requires writable /dev/shm
    }
    let other = shm.join(format!("hermes-exdev-test-{}", std::process::id()));
    if std::fs::create_dir(&other).is_err() {
        return;
    }
    (|| {
        let real = other.join("config.yaml");
        std::fs::write(&real, "old\n").unwrap();
        let td = tempfile::TempDir::new().unwrap();
        let dev_other = std::fs::metadata(&real).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let dev_tmp = std::fs::metadata(td.path()).unwrap();
            if dev_other.dev() == dev_tmp.dev() {
                return; // same filesystem here
            }
        }
        let link = td.path().join("config.yaml");
        symlink(&real, &link).unwrap();
        let tmp = write_tmp(td.path(), "new\n");
        let out = atomic_replace(&tmp, &link).expect("replace");
        assert_eq!(out, real);
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new\n");
        assert!(!tmp.exists());
    })();
    let _ = std::fs::remove_dir_all(&other);
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".wprobe{}", std::process::id()));
    let ok = std::fs::File::create(&probe).is_ok();
    if ok {
        let _ = std::fs::remove_file(&probe);
    }
    ok
}

// ── Windows contended renames (cross-platform state machine via seams) ────

fn sharing_error(winerror: i32) -> std::io::Error {
    std::io::Error::from_raw_os_error(winerror)
}

/// Collapse the jittered backoff so retry tests don't sleep ~1.2s.
fn fast_replace_retries() {
    set_replace_retry_delays_for_test(0.001, 0.001);
}

#[test]
fn oracle_contended_rename_retries_then_rewrites_in_place() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_contended_rename_retries_then_rewrites_in_place (winerror 5/32/33):
    // held target → full retry budget → in-place rewrite lands the write.
    for winerror in [5, 32, 33] {
        let td = tempfile::TempDir::new().unwrap();
        let target = td.path().join("gateway_state.json");
        std::fs::write(&target, "old").unwrap();
        let tmp = write_tmp(td.path(), "new");

        let attempts: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        set_replace_hook_for_test(Some(Box::new({
            let attempts = attempts.clone();
            move |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> {
                *attempts.lock().unwrap() += 1;
                Err(sharing_error(winerror))
            }
        })));
        force_windows_contended_for_test(true);
        fast_replace_retries();

        let out = atomic_replace(&tmp, &target).expect("rewrite lands");
        reset_replace_hook_for_test();
        force_windows_contended_for_test(false);
        reset_replace_retry_delays_for_test();

        assert_eq!(out, target, "winerror {winerror}");
        assert_eq!(
            *attempts.lock().unwrap(),
            1 + REPLACE_RETRY_ATTEMPTS,
            "winerror {winerror}: initial attempt + bounded retries"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert!(!tmp.exists(), "rewrite consumed the temp");
    }
}

#[test]
fn oracle_contended_rename_retry_wins_keeps_write_atomic() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_contended_rename_retry_wins_keeps_write_atomic — a reader that
    // lets go inside the budget: rename wins on a later retry, no fallback.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("gateway_state.json");
    std::fs::write(&target, "old").unwrap();
    let tmp = write_tmp(td.path(), "new");

    let calls: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    set_replace_hook_for_test(Some(Box::new({
        let calls = calls.clone();
        move |t: &Path, d: &Path| -> std::io::Result<PathBuf> {
            let n = {
                let mut c = calls.lock().unwrap();
                *c += 1;
                *c
            };
            if n <= 2 {
                return Err(sharing_error(5));
            }
            std::fs::rename(t, d)?;
            Ok(d.to_path_buf())
        }
    })));
    force_windows_contended_for_test(true);
    fast_replace_retries();

    let out = atomic_replace(&tmp, &target).expect("replace");
    let n = *calls.lock().unwrap();
    reset_replace_hook_for_test();
    force_windows_contended_for_test(false);
    reset_replace_retry_delays_for_test();

    assert_eq!(out, target);
    assert_eq!(n, 3, "two contended failures then a winning rename");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    assert!(!tmp.exists());
}

#[test]
fn oracle_genuine_denial_propagates_after_budget() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_genuine_denial_propagates_after_budget — the in-place rewrite's
    // open fails too; the error surfaces and the pending temp survives.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("denied.json");
    std::fs::write(&target, "old").unwrap();
    let tmp = write_tmp(td.path(), "new");

    set_replace_hook_for_test(Some(Box::new(
        |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> { Err(sharing_error(5)) },
    )));
    force_windows_contended_for_test(true);
    fast_replace_retries();

    // A directory with no write permission cannot host the rewrite open…
    // simpler: make the TARGET a directory so O_WRONLY open fails EISDIR.
    let dir_target = td.path().join("denied_as_dir");
    std::fs::create_dir(&dir_target).unwrap();
    let err = atomic_replace(&tmp, &dir_target).expect_err("must surface");
    reset_replace_hook_for_test();
    force_windows_contended_for_test(false);
    reset_replace_retry_delays_for_test();

    let _ = err; // rewrite failed (EISDIR/EACCES) → propagated
    assert!(
        tmp.exists(),
        "the pending write must survive for the caller"
    );
}

#[test]
fn oracle_contended_retry_switching_to_exdev_uses_copy_fallback() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_contended_retry_switching_to_exdev_uses_copy_fallback — EXDEV
    // never clears on retry: stop the budget, take the copy.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("target.json");
    std::fs::write(&target, "old").unwrap();
    let tmp = write_tmp(td.path(), "new");

    let calls: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    set_replace_hook_for_test(Some(Box::new({
        let calls = calls.clone();
        move |t: &Path, d: &Path| -> std::io::Result<PathBuf> {
            let n = {
                let mut c = calls.lock().unwrap();
                *c += 1;
                *c
            };
            if n == 1 {
                return Err(sharing_error(5));
            }
            // Second call: EXDEV.
            let _ = (t, d);
            Err(std::io::Error::from_raw_os_error(18))
        }
    })));
    force_windows_contended_for_test(true);
    fast_replace_retries();

    let out = atomic_replace(&tmp, &target).expect("replace");
    let n = *calls.lock().unwrap();
    reset_replace_hook_for_test();
    force_windows_contended_for_test(false);
    reset_replace_retry_delays_for_test();

    assert_eq!(out, target);
    assert_eq!(n, 2, "one contended attempt + one EXDEV retry → copy");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    assert!(!tmp.exists());
}

#[test]
fn oracle_non_contended_oserror_propagates_without_retry() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_non_contended_oserror_propagates_without_retry — ENOSPC is not
    // a contention code: one attempt, no fallback, temp left for caller.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("config.yaml");
    std::fs::write(&target, "old").unwrap();
    let tmp = write_tmp(td.path(), "new");

    let calls: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    set_replace_hook_for_test(Some(Box::new({
        let calls = calls.clone();
        move |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> {
            *calls.lock().unwrap() += 1;
            Err(std::io::Error::from_raw_os_error(28)) // ENOSPC
        }
    })));
    force_windows_contended_for_test(true);

    let err = atomic_replace(&tmp, &target).expect_err("ENOSPC surfaces");
    let n = *calls.lock().unwrap();
    reset_replace_hook_for_test();
    force_windows_contended_for_test(false);

    assert_eq!(err.raw_os_error(), Some(28));
    assert_eq!(n, 1, "a permanent error must not be retried");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    assert!(tmp.exists());
}

#[test]
fn oracle_posix_eacces_propagates_without_retry() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_posix_eacces_propagates_without_retry — on POSIX EACCES means
    // directory permissions: no retry loop regardless of the winerror table.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("config.yaml");
    std::fs::write(&target, "old").unwrap();
    let tmp = write_tmp(td.path(), "new");

    let calls: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    set_replace_hook_for_test(Some(Box::new({
        let calls = calls.clone();
        move |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> {
            *calls.lock().unwrap() += 1;
            Err(std::io::Error::from_raw_os_error(13)) // EACCES
        }
    })));
    // windows flag stays false — default.

    let err = atomic_replace(&tmp, &target).expect_err("EACCES surfaces");
    let n = *calls.lock().unwrap();
    reset_replace_hook_for_test();

    assert_eq!(err.raw_os_error(), Some(13));
    assert_eq!(n, 1, "POSIX must not gain a retry loop");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    assert!(tmp.exists());
}

#[test]
fn oracle_in_place_rewrite_never_exposes_a_truncated_file() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_in_place_rewrite_never_exposes_a_truncated_file — write through
    // (no truncate-then-fill) and shrinking rewrites drop the tail.
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("auth.json");
    std::fs::write(&target, "A".repeat(5000)).unwrap();

    let tmp = write_tmp(td.path(), &"B".repeat(5000));
    hermes_utils::rewrite_in_place(&tmp, &target).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "B".repeat(5000));
    assert!(!tmp.exists());

    // Shrinking rewrite: set_len must drop the tail.
    let tmp = write_tmp(td.path(), &"C".repeat(10));
    hermes_utils::rewrite_in_place(&tmp, &target).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "C".repeat(10));
    assert!(!tmp.exists());
}

#[cfg(unix)]
#[test]
fn oracle_symlinked_target_survives_a_contended_rename() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_symlinked_target_survives_a_contended_rename — #16743 invariant
    // holds on the contended path too.
    use std::os::unix::fs::symlink;
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("real.yaml");
    std::fs::write(&real, "old\n").unwrap();
    let link = td.path().join("config.yaml");
    symlink(&real, &link).unwrap();
    let tmp = write_tmp(td.path(), "new\n");

    set_replace_hook_for_test(Some(Box::new(
        |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> { Err(sharing_error(5)) },
    )));
    force_windows_contended_for_test(true);
    fast_replace_retries();

    let out = atomic_replace(&tmp, &link).expect("replace");
    reset_replace_hook_for_test();
    force_windows_contended_for_test(false);
    reset_replace_retry_delays_for_test();

    assert_eq!(out, real);
    assert!(link.is_symlink(), "symlink survives the rewrite fallback");
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "new\n");
    assert!(!tmp.exists());
}

// ── test_atomic_json_writers_unified.py (utils-owned halves) ─────────────

#[test]
fn oracle_failed_replace_keeps_old_bytes_and_no_temp() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The shared contract: a failed replace leaves the previous file
    // byte-identical AND leaves no temp file behind (the interrupt-safe
    // cleanup only the canonical helper guarantees).
    let td = tempfile::TempDir::new().unwrap();
    let target = td.path().join("sessions.json");
    std::fs::write(&target, r#"{"old": true}"#).unwrap();

    set_replace_hook_for_test(Some(Box::new(
        |_t: &Path, _d: &Path| -> std::io::Result<PathBuf> {
            Err(std::io::Error::other("simulated disk full"))
        },
    )));
    let err = atomic_json_write(&target, &serde_json::json!({"k": "v"}), 2, None, false)
        .expect_err("replace failure surfaces");
    reset_replace_hook_for_test();

    assert!(err.to_string().contains("disk full"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        r#"{"old": true}"#,
        "old bytes untouched"
    );
    let leftovers: Vec<_> = std::fs::read_dir(td.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() != "sessions.json")
        .collect();
    assert!(leftovers.is_empty(), "no temp left: {leftovers:?}");
}

#[cfg(unix)]
#[test]
fn oracle_new_non_secret_json_follows_umask_secret_and_existing_hold() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_new_non_secret_file_follows_umask_while_secret_and_existing_modes_hold.
    use std::os::unix::fs::PermissionsExt;
    let td = tempfile::TempDir::new().unwrap();
    unsafe {
        let old = libc::umask(0o022);

        let fresh = td.path().join("cache.json");
        atomic_json_write(&fresh, &serde_json::json!({"a": 1}), 2, None, false).unwrap();
        assert_eq!(
            std::fs::metadata(&fresh).unwrap().permissions().mode() & 0o777,
            0o644,
            "new non-secret file must not inherit mkstemp's 0600"
        );

        let secret = td.path().join("creds.json");
        atomic_json_write(
            &secret,
            &serde_json::json!({"token": "x"}),
            2,
            Some(0o600),
            false,
        )
        .unwrap();
        assert_eq!(
            std::fs::metadata(&secret).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let existing = td.path().join("state.json");
        std::fs::write(&existing, "{}").unwrap();
        std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o640)).unwrap();
        atomic_json_write(&existing, &serde_json::json!({"b": 2}), 2, None, false).unwrap();
        assert_eq!(
            std::fs::metadata(&existing).unwrap().permissions().mode() & 0o777,
            0o640,
            "existing mode preserved when no mode= given"
        );

        libc::umask(old);
    }
}

#[test]
fn oracle_atomic_json_indent_width_is_honored() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // json.dumps(indent=N) contract: N spaces per level (indent=0 → newlines
    // only). The old port ignored the parameter (documented divergence);
    // it is now honored.
    let td = tempfile::TempDir::new().unwrap();
    let p2 = td.path().join("two.json");
    atomic_json_write(&p2, &serde_json::json!({"a": {"b": 1}}), 2, None, false).unwrap();
    let text = std::fs::read_to_string(&p2).unwrap();
    assert!(
        text.contains("\n  \"a\":"),
        "2-space indent level 1: {text:?}"
    );
    assert!(
        text.contains("\n    \"b\": 1"),
        "2-space indent level 2: {text:?}"
    );

    let p4 = td.path().join("four.json");
    atomic_json_write(&p4, &serde_json::json!({"a": {"b": 1}}), 4, None, false).unwrap();
    let text4 = std::fs::read_to_string(&p4).unwrap();
    assert!(
        text4.contains("\n    \"a\":"),
        "4-space indent level 1: {text4:?}"
    );
    assert!(
        text4.contains("\n        \"b\": 1"),
        "4-space indent level 2: {text4:?}"
    );
}

#[cfg(unix)]
#[test]
fn oracle_atomic_json_write_preserves_symlink_and_permissions() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_atomic_json_write_preserves_symlink + …_permissions.
    use std::os::unix::fs::{symlink, PermissionsExt};
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("real.json");
    std::fs::write(&real, "{}").unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o644)).unwrap();
    let link = td.path().join("link.json");
    symlink(&real, &link).unwrap();

    atomic_json_write(
        &link,
        &serde_json::json!({"hello": "world"}),
        2,
        None,
        false,
    )
    .unwrap();

    assert!(link.is_symlink());
    let loaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&real).unwrap()).unwrap();
    assert_eq!(loaded, serde_json::json!({"hello": "world"}));
    assert_eq!(
        std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o644,
        "permissions drifted after symlinked write"
    );
}

// ── test_yaml_indent_consistency.py (#31999) ─────────────────────────────

#[test]
fn oracle_atomic_yaml_write_produces_2_indent_lists() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let data = serde_json::json!({
        "custom_providers": [{"name": "Test", "base_url": "https://example.com"}],
    });
    let path = td.path().join("config.yaml");
    atomic_yaml_write(&path, &data, false, None, None).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("  - "),
        "Expected 2-indent list in file, got:\n{content}"
    );
}

#[test]
fn oracle_atomic_yaml_write_preserves_unicode() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("config.yaml");
    atomic_yaml_write(
        &path,
        &serde_json::json!({"name": "Tëst Näme"}),
        false,
        None,
        None,
    )
    .unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Tëst Näme"), "{content}");
}

#[test]
fn oracle_atomic_yaml_write_is_atomic() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("config.yaml");
    atomic_yaml_write(
        &path,
        &serde_json::json!({"key": "value"}),
        false,
        None,
        None,
    )
    .unwrap();
    assert!(path.exists());
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .trim()
        .ends_with("value"));
    // No leftover temp files (`.config_*.tmp`).
    let leftovers: Vec<_> = std::fs::read_dir(td.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "leftover temps: {leftovers:?}");
}

#[test]
fn oracle_atomic_yaml_write_roundtrips_through_safe_load() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // TestRoundtripConsistency — the written file re-parses to the same values.
    let td = tempfile::TempDir::new().unwrap();
    let data = serde_json::json!({
        "custom_providers": [
            {"name": "Provider A", "base_url": "https://a.example.com"},
            {"name": "Provider B", "base_url": "https://b.example.com"},
        ],
        "fallback_providers": ["backup1", "backup2"],
    });
    let path = td.path().join("config.yaml");
    atomic_yaml_write(&path, &data, false, None, None).unwrap();
    let loaded = fast_safe_load(&std::fs::read_to_string(&path).unwrap()).expect("re-parse");
    let as_json = serde_json::to_value(loaded).unwrap();
    assert_eq!(
        as_json["custom_providers"][0]["name"],
        serde_json::json!("Provider A")
    );
    assert_eq!(
        as_json["fallback_providers"],
        serde_json::json!(["backup1", "backup2"])
    );
}

#[cfg(unix)]
#[test]
fn oracle_atomic_yaml_write_restores_owner_on_real_symlink_target() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_atomic_yaml_write_restores_owner_on_real_symlink_target — forced
    // uid/gid lands on the real file through the link (no root needed).
    use std::os::unix::fs::symlink;
    let td = tempfile::TempDir::new().unwrap();
    let real = td.path().join("config.yaml");
    std::fs::write(&real, "old: true\n").unwrap();
    let link = td.path().join("link.yaml");
    symlink(&real, &link).unwrap();

    let chowns: Arc<Mutex<Vec<(PathBuf, u32, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    set_owner_seams_for_test(
        Some(|_p: &Path| Some((123, 456))),
        Some(Box::new({
            let rec = chowns.clone();
            move |p: &Path, uid, gid| {
                rec.lock().unwrap().push((p.to_path_buf(), uid, gid));
            }
        })),
    );
    atomic_yaml_write(&link, &serde_json::json!({"new": true}), false, None, None).unwrap();
    reset_owner_seams_for_test();

    assert_eq!(*chowns.lock().unwrap(), vec![(real, 123, 456)]);
}

// ── test_utils_atomic_roundtrip_yaml_save.py ──────────────────────────────

fn save(path: &Path, state: serde_json::Value) -> Result<(), std::io::Error> {
    atomic_roundtrip_yaml_save(path, &state)
}

fn load_yaml(path: &Path) -> serde_yaml::Value {
    fast_safe_load(&std::fs::read_to_string(path).unwrap()).expect("re-parse")
}

#[test]
fn oracle_roundtrip_save_creates_file_when_missing() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    save(
        &config,
        serde_json::json!({"model": {"default": "test-model"}}),
    )
    .unwrap();
    assert!(config.exists());
    assert_eq!(
        load_yaml(&config)["model"]["default"].as_str(),
        Some("test-model")
    );
}

#[test]
fn oracle_roundtrip_save_preserves_top_level_key_order() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(
        &config,
        "model:\n  default: claude-opus-4-7\nproviders: {}\nagent:\n  max_turns: 90\ndisplay:\n  skin: default\n",
    )
    .unwrap();
    // Caller keys in ALPHABETICAL order — the file must keep its author order.
    save(
        &config,
        serde_json::json!({
            "agent": {"max_turns": 100},
            "display": {"skin": "mono"},
            "model": {"default": "claude-opus-4-7"},
            "providers": {},
        }),
    )
    .unwrap();

    let text = std::fs::read_to_string(&config).unwrap();
    let top_keys: Vec<&str> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(' ') && !l.starts_with('#'))
        .map(|l| l.split(':').next().unwrap())
        .collect();
    assert_eq!(
        top_keys,
        ["model", "providers", "agent", "display"],
        "original order, not alphabetical: {text}"
    );
    // Values took the new state.
    let loaded = load_yaml(&config);
    assert_eq!(loaded["agent"]["max_turns"].as_i64(), Some(100));
    assert_eq!(loaded["display"]["skin"].as_str(), Some("mono"));
}

#[test]
fn oracle_roundtrip_save_preserves_comments() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(
        &config,
        "# header comment\nmodel:\n  # inline note\n  default: claude-opus-4-7\ndisplay:\n  skin: default  # trailing note\n",
    )
    .unwrap();
    save(
        &config,
        serde_json::json!({
            "model": {"default": "claude-opus-4-7"},
            "display": {"skin": "mono"},
        }),
    )
    .unwrap();

    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("# header comment"), "{text}");
    assert!(text.contains("# inline note"), "{text}");
    assert!(text.contains("# trailing note"), "{text}");
    assert_eq!(load_yaml(&config)["display"]["skin"].as_str(), Some("mono"));
}

#[test]
fn oracle_roundtrip_save_preserves_readable_unicode() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(
        &config,
        "agent:\n  personalities:\n    catgirl: \"nya (=^･ω･^=) 你好\"\ndisplay:\n  skin: default\n",
    )
    .unwrap();
    save(
        &config,
        serde_json::json!({
            "agent": {"personalities": {"catgirl": "nya (=^･ω･^=) 你好"}},
            "display": {"skin": "mono"},
        }),
    )
    .unwrap();

    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("你好"), "{text}");
    assert!(text.contains("(=^･ω･^=)"), "{text}");
    assert!(!text.contains("\\u4f60"), "{text}");
    assert!(!text.contains("\\u30CE"), "{text}");
}

#[test]
fn oracle_roundtrip_save_appends_new_keys() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(&config, "model:\n  default: test-model\n").unwrap();
    save(
        &config,
        serde_json::json!({
            "model": {"default": "test-model"},
            "display": {"personality": "noir"},
            "agent": {"system_prompt": "you are noir"},
        }),
    )
    .unwrap();

    let loaded = load_yaml(&config);
    assert_eq!(loaded["model"]["default"].as_str(), Some("test-model"));
    assert_eq!(loaded["display"]["personality"].as_str(), Some("noir"));
    assert_eq!(
        loaded["agent"]["system_prompt"].as_str(),
        Some("you are noir")
    );
}

#[test]
fn oracle_roundtrip_save_deletes_keys_missing_from_new_state() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(
        &config,
        "model:\n  default: test-model\ncustom_prompt: 'old prompt'\n",
    )
    .unwrap();
    save(
        &config,
        serde_json::json!({"model": {"default": "test-model"}}),
    )
    .unwrap();

    let loaded = load_yaml(&config);
    assert!(loaded.get("custom_prompt").is_none(), "explicit absence");
    assert_eq!(loaded["model"]["default"].as_str(), Some("test-model"));
}

#[test]
fn oracle_roundtrip_save_overwrites_scalar_and_list_wholesale() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(&config, "display:\n  personality: noir\n").unwrap();
    save(
        &config,
        serde_json::json!({"display": {"personality": "kawaii"}}),
    )
    .unwrap();
    assert_eq!(
        load_yaml(&config)["display"]["personality"].as_str(),
        Some("kawaii")
    );

    let config2 = td.path().join("lists.yaml");
    std::fs::write(&config2, "toolsets:\n  - one\n  - two\n").unwrap();
    save(&config2, serde_json::json!({"toolsets": ["three"]})).unwrap();
    assert_eq!(
        load_yaml(&config2)["toolsets"],
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("three".into())])
    );
}

#[test]
fn oracle_roundtrip_save_recurses_into_nested_dicts() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(
        &config,
        "display:\n  skin: default\n  personality: noir\n  compact: false\n",
    )
    .unwrap();
    save(
        &config,
        serde_json::json!({
            "display": {"skin": "default", "personality": "kawaii", "compact": false}
        }),
    )
    .unwrap();

    let loaded = load_yaml(&config);
    assert_eq!(loaded["display"]["skin"].as_str(), Some("default"));
    assert_eq!(loaded["display"]["personality"].as_str(), Some("kawaii"));
    assert_eq!(loaded["display"]["compact"].as_bool(), Some(false));
}

#[cfg(unix)]
#[test]
fn oracle_roundtrip_save_refuses_unreadable_existing_config() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_refuses_to_overwrite_unreadable_existing_config — fail closed
    // with the "this change was not saved" contract, original bytes kept.
    use std::os::unix::fs::PermissionsExt;
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    let original = "model:\n  default: test-model\n";
    std::fs::write(&config, original).unwrap();
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o000)).unwrap();

    let err = save(
        &config,
        serde_json::json!({"model": {"default": "replacement"}}),
    )
    .expect_err("must refuse");
    // Restore before reading/removing (we are not root; 000 blocks us too).
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        err.to_string().contains("this change was not saved"),
        "{}",
        err
    );
    assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn oracle_roundtrip_save_restores_owner() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // test_restores_owner — forced uid/gid carried across the swap.
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    std::fs::write(&config, "model:\n  default: test-model\n").unwrap();

    let chowns: Arc<Mutex<Vec<(PathBuf, u32, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    set_owner_seams_for_test(
        Some(|_p: &Path| Some((345, 678))),
        Some(Box::new({
            let rec = chowns.clone();
            move |p: &Path, uid, gid| {
                rec.lock().unwrap().push((p.to_path_buf(), uid, gid));
            }
        })),
    );
    save(
        &config,
        serde_json::json!({"model": {"default": "updated-model"}}),
    )
    .unwrap();
    reset_owner_seams_for_test();

    assert_eq!(*chowns.lock().unwrap(), vec![(config.clone(), 345, 678)]);
    assert_eq!(
        load_yaml(&config)["model"]["default"].as_str(),
        Some("updated-model")
    );
}

#[test]
fn oracle_roundtrip_save_quotes_yaml11_ambiguous_strings() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Probe: ruamel YAML-1.2 would emit `off` unquoted and a later
    // safe_load (1.1) would flip it to False; the save path forces
    // double quotes for the ambiguous word set.
    let td = tempfile::TempDir::new().unwrap();
    let config = td.path().join("config.yaml");
    save(
        &config,
        serde_json::json!({"a": "off", "b": "yes", "c": "plain"}),
    )
    .unwrap();
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("a: \"off\""), "{text}");
    assert!(text.contains("b: \"yes\""), "{text}");
    assert!(text.contains("c: plain"), "{text}");
}

// ── new-module fns: file_signature / read_json_or_empty / fsync_directory ─

#[test]
fn oracle_file_signature_quad_and_read_json_or_empty() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // file_signature: (mtime_ns, size, ino, ctime_ns) — 4-tuple change key.
    let td = tempfile::TempDir::new().unwrap();
    let p = td.path().join("sig.txt");
    std::fs::write(&p, "hello").unwrap();
    let sig = file_signature(&p).expect("stat works");
    assert_eq!(sig.1, 5, "size");
    assert!(sig.0 > 0, "mtime_ns");

    // read_json_or_empty: missing/non-object/malformed → {}; object → data;
    // a Windows-editor BOM must not wipe the config (utf-8-sig).
    let j = td.path().join("x.json");
    assert!(read_json_or_empty(&j).is_object());
    assert!(read_json_or_empty(&j).as_object().unwrap().is_empty());
    std::fs::write(&j, "[1]").unwrap();
    assert!(
        read_json_or_empty(&j).as_object().unwrap().is_empty(),
        "non-object maps to {{}}"
    );
    std::fs::write(&j, r#"{"a":1}"#).unwrap();
    assert_eq!(read_json_or_empty(&j)["a"].as_i64(), Some(1));
    std::fs::write(&j, "\u{feff}{\"b\":2}").unwrap();
    assert_eq!(
        read_json_or_empty(&j)["b"].as_i64(),
        Some(2),
        "utf-8-sig BOM stripped"
    );

    // fsync_directory: no-op without panicking (best-effort).
    fsync_directory(td.path());
}

#[cfg(unix)]
#[test]
fn oracle_atomic_write_bytes_secret_shape() {
    let _seam = SEAM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::fs::PermissionsExt;
    // Bytes variant: explicit mode pins the final bits (encrypted blobs).
    let td = tempfile::TempDir::new().unwrap();
    let p = td.path().join("blob.bin");
    atomic_write_bytes(&p, b"\x00\x01secret", Some(0o600), false).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"\x00\x01secret");
    assert_eq!(
        std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
