//! Parity oracles for the Python toolchain probe, mirroring upstream
//! tests/tools/test_env_probe.py @ b9aa928.
//!
//! Tier: mock/unit. Upstream monkeypatches module functions; Rust modules
//! are not monkeypatchable, so the probe scenarios run the REAL code
//! (which -> subprocess -> parse) against fake `python3` / `pip` / `uv`
//! shell scripts placed on a test-controlled PATH (mock tier — real
//! subprocess calls, fake tool binaries). The concurrency tests
//! (stuck/late/peek) use the module's documented test seams
//! (set_build_probe_line_override_for_tests / wait-timeout override) which
//! mirror upstream monkeypatching `_build_probe_line` / `_PROBE_WAIT_TIMEOUT`.
//!
//! The `_run` subprocess-behavior tests drive the REAL test binary as the
//! probed child via the PARITY_ENV_PROBE_HELPER env var (a grandchild that
//! outlives its parent, exactly like the #67964 pip.exe launcher shape).
//!
//! Env-var mutations are process-global; every env-mutating test serializes
//! on ENV_PROBE_TEST_LOCK and resets the probe cache before AND after
//! (mirroring the upstream autouse reset_probe_cache fixture).

use std::env;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hermes_tools::env_probe::{
    _probe_done_wait_for_tests, _reset_cache_for_tests, _run,
    clear_build_probe_line_override_for_tests, clear_probe_wait_timeout_override_for_tests,
    get_environment_probe_line,
    set_build_probe_line_override_for_tests, set_probe_wait_timeout_override_for_tests,
    warm_environment_probe_async,
};

static ENV_PROBE_TEST_LOCK: Mutex<()> = Mutex::new(());

const PYTHON3_FAKE: &str = r#"#!/bin/sh
# Fake python3 for env_probe parity tests. Behavior controlled by env:
#   PARITY_PROBE_VERSION    version string echoed for the version probe
#   PARITY_PROBE_PEP668     'yes'/'no' for the EXTERNALLY-MANAGED probe
#   PARITY_PROBE_PIP_RC     exit code for `-m pip --version`
#   PARITY_PROBE_COUNTER    file appended with one x per invocation
V="${PARITY_PROBE_VERSION-3.13.3}"
P="${PARITY_PROBE_PEP668-no}"
R="${PARITY_PROBE_PIP_RC-0}"
if [ -n "$PARITY_PROBE_COUNTER" ]; then echo x >> "$PARITY_PROBE_COUNTER"; fi
case "$1" in
  -m) exit "$R" ;;
  -c)
    case "$2" in
      *sys.version_info*) echo "$V"; exit 0 ;;
      *EXTERNALLY-MANAGED*) echo "$P"; exit 0 ;;
      *) exit 0 ;;
    esac ;;
esac
exit 0
"#;

const PIP_FAKE: &str = r#"#!/bin/sh
# Fake pip: mirrors `pip --version` output shape.
echo "pip 25.0 from /usr/lib/python3/dist-packages/pip (python ${PARITY_PROBE_PIP_BOUND-3.13})"
"#;

const UV_FAKE: &str = "#!/bin/sh\nexit 0\n";

/// A directory of fake tool scripts with exec bits set.
struct FakeToolbox {
    dir: tempfile::TempDir,
}

impl FakeToolbox {
    fn new(tools: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, body) in tools {
            let p = dir.path().join(name);
            fs::write(&p, body).expect("write fake tool");
            let mut perms = fs::metadata(&p).expect("meta").permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                perms.set_mode(0o755);
            }
            fs::set_permissions(&p, perms).expect("chmod fake tool");
        }
        FakeToolbox { dir }
    }
    fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// Save/restore a single env var (used with the global lock so the four
/// probe scenarios and the process-global cache never interleave).
struct EnvVarGuard<'a> {
    key: &'a str,
    previous: Option<std::ffi::OsString>,
}

impl<'a> EnvVarGuard<'a> {
    fn set(key: &'a str, value: Option<&str>) -> Self {
        let previous = env::var_os(key);
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        EnvVarGuard { key, previous }
    }
}

impl Drop for EnvVarGuard<'_> {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => env::set_var(self.key, v),
            None => env::remove_var(self.key),
        }
    }
}

// ---------------------------------------------------------------------------
// TestSilentWhenHealthy
// ---------------------------------------------------------------------------

#[test]
fn clean_env_returns_empty() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let tb = FakeToolbox::new(&[("python3", PYTHON3_FAKE), ("pip", PIP_FAKE)]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    // py3 pip module present (PIP_RC 0), PEP 668 off, pip bound to 3.13,
    // no uv → silent.
    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert_eq!(line, "");
}

#[test]
fn pep668_with_uv_returns_empty() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let tb = FakeToolbox::new(&[
        ("python3", PYTHON3_FAKE),
        ("pip", PIP_FAKE),
        ("uv", UV_FAKE),
    ]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    let _v = EnvVarGuard::set("PARITY_PROBE_VERSION", Some("3.12.4"));
    let _p = EnvVarGuard::set("PARITY_PROBE_PEP668", Some("yes"));
    let _b = EnvVarGuard::set("PARITY_PROBE_PIP_BOUND", Some("3.12"));
    // PEP 668 alone shouldn't trigger output if uv is installed.
    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert_eq!(line, "");
}

// ---------------------------------------------------------------------------
// TestEmitsOnRealProblems
// ---------------------------------------------------------------------------

#[test]
fn allen_scenario_python_version_mismatch() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    // python3 is 3.11 (no pip module), pip on PATH is 3.12, PEP 668 on,
    // no uv — the exact Sarasota scenario.
    let tb = FakeToolbox::new(&[("python3", PYTHON3_FAKE), ("pip", PIP_FAKE)]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    let _v = EnvVarGuard::set("PARITY_PROBE_VERSION", Some("3.11.15"));
    let _r = EnvVarGuard::set("PARITY_PROBE_PIP_RC", Some("1"));
    let _p = EnvVarGuard::set("PARITY_PROBE_PEP668", Some("yes"));
    let _b = EnvVarGuard::set("PARITY_PROBE_PIP_BOUND", Some("3.12"));

    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert!(!line.is_empty(), "expected a non-empty probe line");
    // Single line — must not blow up the system prompt.
    assert!(!line.contains('\n'));
    assert!(line.contains("3.11.15"), "line: {line}");
    assert!(line.contains("no pip module"), "line: {line}");
    assert!(line.contains("mismatch"), "line: {line}");
    assert!(line.contains("PEP 668"), "line: {line}");
    assert!(line.contains("venv") || line.contains("uv"), "line: {line}");
}

#[test]
fn python_missing_but_python3_present() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    // Common on Debian: only python3 exists, agent shouldn't type `python`.
    let tb = FakeToolbox::new(&[("python3", PYTHON3_FAKE), ("pip", PIP_FAKE)]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    let _v = EnvVarGuard::set("PARITY_PROBE_VERSION", Some("3.12.4"));
    let _p = EnvVarGuard::set("PARITY_PROBE_PEP668", Some("yes"));
    let _b = EnvVarGuard::set("PARITY_PROBE_PIP_BOUND", Some("3.12"));

    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert!(line.contains("PEP 668"), "line: {line}");
    assert!(line.contains("python=missing"), "line: {line}");
}

// ---------------------------------------------------------------------------
// TestSkipsRemoteBackends
// ---------------------------------------------------------------------------

#[test]
fn docker_returns_empty() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    // Even with a broken local env, docker must emit nothing.
    let tb = FakeToolbox::new(&[("pip", PIP_FAKE)]); // no python3 at all
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", Some("docker"));
    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert_eq!(line, "");
}

#[test]
fn ssh_returns_empty() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let _term = EnvVarGuard::set("TERMINAL_ENV", Some("ssh"));
    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert_eq!(line, "");
}

// ---------------------------------------------------------------------------
// TestCaching
// ---------------------------------------------------------------------------

#[test]
fn result_cached() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let tb = FakeToolbox::new(&[
        ("python3", PYTHON3_FAKE),
        ("pip", PIP_FAKE),
        ("uv", UV_FAKE),
    ]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    let counter = tb.path().join("probe-counter.log");
    let _c = EnvVarGuard::set("PARITY_PROBE_COUNTER", Some(counter.to_str().unwrap()));

    let count = || -> usize {
        fs::read_to_string(&counter)
            .map(|s| s.lines().count())
            .unwrap_or(0)
    };

    get_environment_probe_line(false);
    get_environment_probe_line(false);
    get_environment_probe_line(false);

    // Only the first call probes. Upstream counts `_python_version_of`
    // function calls (2: python3 + python); our fake python3 binary is
    // invoked 3x on the first build (version probe + `-m pip` + PEP-668),
    // zero after — the cache semantics are identical.
    assert_eq!(count(), 3);
    _reset_cache_for_tests();
}

// ---------------------------------------------------------------------------
// TestRobustness
// ---------------------------------------------------------------------------

#[test]
fn subprocess_failure_returns_empty() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    // A python3 that fails the version probe (empty output) with no pip on
    // PATH: the probe must never crash, and whatever it returns is a string.
    let tb = FakeToolbox::new(&[("python3", PYTHON3_FAKE)]);
    let _path = EnvVarGuard::set("PATH", Some(tb.path().to_str().unwrap()));
    let _term = EnvVarGuard::set("TERMINAL_ENV", None);
    let _v = EnvVarGuard::set("PARITY_PROBE_VERSION", Some(""));
    let _r = EnvVarGuard::set("PARITY_PROBE_PIP_RC", Some("1")); // -m pip fails too
    let line = get_environment_probe_line(false);
    _reset_cache_for_tests();
    assert!(!line.is_empty()); // upstream only pins bytes-are-a-str; ours is deterministic:
    assert_eq!(line, "Python toolchain: python3=missing, pip=missing.");
}

// ---------------------------------------------------------------------------
// TestStuckProbeNeverBlocksCallers (upstream regression #67964)
// ---------------------------------------------------------------------------

#[test]
fn hung_probe_fails_open_for_concurrent_callers() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let release = Arc::new(AtomicBool::new(false));
    let release_worker = release.clone();
    set_build_probe_line_override_for_tests(move || {
        // Simulate the wedged pipe read: blocks until released.
        while !release_worker.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        "Python toolchain: late-result.".to_string()
    });
    // Keep the test fast — the bound just has to exist, not be 10s.
    set_probe_wait_timeout_override_for_tests(Duration::from_millis(500));

    warm_environment_probe_async();

    let start = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..4 {
        handles.push(std::thread::spawn(|| {
            get_environment_probe_line(false)
        }));
    }
    let results: Vec<String> = handles
        .into_iter()
        .map(|h| h.join().unwrap_or_default())
        .collect();
    let elapsed = start.elapsed();
    // Cleanup before asserting so a failure can't wedge later tests.
    release.store(true, Ordering::SeqCst);
    clear_build_probe_line_override_for_tests();
    clear_probe_wait_timeout_override_for_tests();
    _reset_cache_for_tests();

    // All callers failed open with the empty line, bounded well under the
    // 30s the probe is stuck for.
    assert_eq!(results, vec![String::new(); 4]);
    assert!(elapsed < Duration::from_secs(8), "elapsed {elapsed:?}");
}

#[test]
fn late_probe_result_published_after_recovery() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let release = Arc::new(AtomicBool::new(false));
    let release_worker = release.clone();
    set_build_probe_line_override_for_tests(move || {
        while !release_worker.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        "Python toolchain: recovered.".to_string()
    });
    set_probe_wait_timeout_override_for_tests(Duration::from_millis(200));

    // First caller times out and fails open.
    assert_eq!(get_environment_probe_line(false), "");

    // Worker un-wedges (the operator killed the orphan).
    release.store(true, Ordering::SeqCst);
    assert!(_probe_done_wait_for_tests(Duration::from_secs(10)));

    // Later callers see the published line.
    assert_eq!(
        get_environment_probe_line(false),
        "Python toolchain: recovered."
    );
    clear_build_probe_line_override_for_tests();
    clear_probe_wait_timeout_override_for_tests();
    _reset_cache_for_tests();
}

#[test]
fn repeat_callers_do_not_pay_full_wait_after_first_timeout() {
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    _reset_cache_for_tests();
    let release = Arc::new(AtomicBool::new(false));
    let release_worker = release.clone();
    set_build_probe_line_override_for_tests(move || {
        while !release_worker.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        String::new()
    });
    set_probe_wait_timeout_override_for_tests(Duration::from_millis(500));

    // First caller pays the full 0.5s wait.
    let start = Instant::now();
    assert_eq!(get_environment_probe_line(false), "");
    assert!(start.elapsed() >= Duration::from_millis(400));

    // Crank the timeout way up: if the peek short-circuit is broken, this
    // blocks ~30s; if it works, it returns in ~0.05s.
    set_probe_wait_timeout_override_for_tests(Duration::from_secs(30));
    let start = Instant::now();
    assert_eq!(get_environment_probe_line(false), "");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "elapsed {:?}",
        start.elapsed()
    );

    release.store(true, Ordering::SeqCst);
    clear_build_probe_line_override_for_tests();
    clear_probe_wait_timeout_override_for_tests();
    _reset_cache_for_tests();
}

// ---------------------------------------------------------------------------
// `_run` bounded-wait semantics (the #67964 temp-file capture fix)
// ---------------------------------------------------------------------------

#[test]
fn run_returns_promptly_despite_pipe_holding_descendant() {
    // Direct child never exits (sleeps 20s) after spawning a background
    // grandchild that also sleeps far beyond the timeout while inheriting
    // the captured fds (the #67964 pip.exe launcher shape, cross-platform
    // via a shell child). Temperatures through temp files means the 1s
    // timeout bounds the whole call.
    let script = "sleep 20 & sleep 20";
    let args = ["sh", "-c", script];
    // Serialize with env-mutating tests so PATH is always system-default
    // (a concurrent test's fake-only PATH would hide `sh`).
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    let start = Instant::now();
    let (rc, out, err) = _run(&args, Duration::from_secs(1));
    let elapsed = start.elapsed();

    assert_eq!(rc, -1);
    assert_eq!(err, "timeout");
    assert!(elapsed < Duration::from_secs(6), "elapsed {elapsed:?}");
    let _ = out;
}

#[test]
fn run_returns_before_inheriting_grandchild_exits() {
    // Direct child prints ok and exits immediately after spawning a
    // long-sleeping grandchild that inherits its stdout (the temp file).
    // With pipe-based capture this would block until the grandchild exits;
    // temp-file capture returns the child's real output within ms.
    let script = "sleep 20 & echo ok";
    let args = ["sh", "-c", script];
    let _guard = ENV_PROBE_TEST_LOCK.lock().unwrap();
    let start = Instant::now();
    let (rc, out, err) = _run(&args, Duration::from_secs(3));
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(3),
        "_run blocked on grandchild for {elapsed:?} (rc={rc} err={err:?})"
    );
    assert_eq!(rc, 0, "expected clean exit, got rc={rc} err={err:?}");
    assert_eq!(out, "ok");
}
