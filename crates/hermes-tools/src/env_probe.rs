//! Local-environment toolchain probe for the system prompt.
//!
//! PARITY: tools/env_probe.py @ b9aa928 (370 LOC, ported 1:1).
//!
//! When the terminal backend is local, Hermes surfaces a single
//! deterministic line about Python tooling state so models don't have to
//! discover it by hitting walls (bundled-venv Python vs login-shell
//! Python, `python3 -m pip` with no pip module, PEP-668 externally
//! managed environments). When the environment looks normal it emits
//! nothing — no token cost. Remote terminal backends are skipped.
//!
//! Caching semantics are part of the contract: the probe runs at most
//! once per process, in exactly ONE background worker thread; callers
//! never run the probe themselves and never wait unboundedly — they block
//! at most `PROBE_WAIT_TIMEOUT` and then fail open with "". A generation
//! counter is bumped on every reset so a stale worker (started before a
//! test reset) can't publish into the fresh generation.
//!
//! Upstream captures subprocess output through temporary files rather
//! than pipes so `timeout` bounds the *whole* call even when a
//! console-script launcher (e.g. `pip.exe`) spawns a descendant that
//! inherits the captured handles and outlives its parent (regression
//! #67964). This port mirrors that with named temp files created in the
//! system temp dir (no reader threads → a lingering grandchild holding
//! the fd can never block the parent's bounded wait).

use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Upper bound a prompt build will wait for the probe. Generous vs the
/// ~0.5s healthy runtime, but finite: prompt construction must proceed.
pub const PROBE_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
/// Once one caller has burned the full wait and given up, later callers
/// stop paying it too — they just peek at the event (50 ms).
const WAIT_ALREADY_TIMED_OUT_PEEK: Duration = Duration::from_millis(50);
/// Per-subprocess timeout used by all probe helpers (mirrors the 3.0
/// default of upstream `_run`).
const RUN_TIMEOUT: Duration = Duration::from_secs(3);

/// Remote backends — keep in sync with agent/prompt_builder.py:
/// `_REMOTE_TERMINAL_BACKENDS`. Duplicated rather than imported to avoid
/// a circular import, exactly like upstream.
const REMOTE_BACKENDS: [&str; 7] = [
    "docker",
    "singularity",
    "modal",
    "daytona",
    "ssh",
    "managed_modal",
    "vercel_sandbox",
];

// Module-level cache. The probe result is deterministic for the lifetime
// of the process. Mirrors the upstream `_CACHE_LOCK` / `_CACHED_LINE` /
// `_PROBE_DONE` / `_PROBE_THREAD` / `_PROBE_GEN` /
// `_WAIT_ALREADY_TIMED_OUT` module globals (Condvar = Event).
#[derive(Default)]
struct ProbeState {
    cached_line: Option<String>, // None = not probed yet; Some(s) = probed
    probe_done: bool,
    probe_thread_started: bool,
    probe_gen: u64,
    wait_already_timed_out: bool,
}

// (Default is derived: cached_line None, everything else false/0 —
// mirrors the upstream module-global initial values.)

static STATE: Mutex<ProbeState> = Mutex::new(ProbeState {
    cached_line: None,
    probe_done: false,
    probe_thread_started: false,
    probe_gen: 0,
    wait_already_timed_out: false,
});
static PROBE_COND: Condvar = Condvar::new();

// Test seams mirroring upstream pytest monkeypatching of
// `_build_probe_line` and `_PROBE_WAIT_TIMEOUT` (Rust cannot reassign
// functions/module constants; the seams are inert unless a test sets
// them).
static TEST_BUILD_OVERRIDE: Mutex<Option<Arc<dyn Fn() -> String + Send + Sync>>> =
    Mutex::new(None);
static TEST_WAIT_TIMEOUT: Mutex<Option<Duration>> = Mutex::new(None);

fn build_probe_line() -> String {
    _build_probe_line()
}

/// Test-only: substitute the probe-body function (mirrors upstream
/// monkeypatching `env_probe._build_probe_line`).
pub fn set_build_probe_line_override_for_tests<F>(f: F)
where
    F: Fn() -> String + Send + Sync + 'static,
{
    *TEST_BUILD_OVERRIDE.lock().unwrap() = Some(Arc::new(f));
}

/// Test-only: clear the probe-body substitution.
pub fn clear_build_probe_line_override_for_tests() {
    *TEST_BUILD_OVERRIDE.lock().unwrap() = None;
}

/// Test-only: shorten `_PROBE_WAIT_TIMEOUT` (mirrors upstream
/// monkeypatching `env_probe._PROBE_WAIT_TIMEOUT`).
pub fn set_probe_wait_timeout_override_for_tests(t: Duration) {
    *TEST_WAIT_TIMEOUT.lock().unwrap() = Some(t);
}

/// Test-only: clear the wait-timeout override.
pub fn clear_probe_wait_timeout_override_for_tests() {
    *TEST_WAIT_TIMEOUT.lock().unwrap() = None;
}

fn probe_wait_timeout() -> Duration {
    TEST_WAIT_TIMEOUT
        .lock()
        .unwrap()
        .unwrap_or(PROBE_WAIT_TIMEOUT)
}

/// Unique named temp file pair for `_run` output capture.
struct TempProbeFiles {
    out: PathBuf,
    err: PathBuf,
}

impl Drop for TempProbeFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.out);
        let _ = fs::remove_file(&self.err);
    }
}

fn temp_probe_file(tag: &str) -> std::io::Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let path = env::temp_dir().join(format!("hermes-env-probe-{pid}-{n}-{nanos}.{tag}"));
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    Ok(path)
}

/// Run a short subprocess.  Returns `(returncode, stdout, stderr)`.
///
/// Failures (binary missing, timeout, OSError) return `(-1, "",
/// "<reason>")`.  Output is captured through temporary files rather than
/// pipes so `timeout` bounds the *whole* call — exactly the upstream
/// #67964 fix (see module doc).
pub fn _run(cmd: &[&str], timeout: Duration) -> (i32, String, String) {
    // Empty command: treat like a launcher failure (defensive; upstream
    // would raise on spawn of an empty argv).
    if cmd.is_empty() {
        return (-1, String::new(), "oserror: empty command".to_string());
    }
    let files = match temp_probe_file("out").and_then(|out| {
        temp_probe_file("err").map(|err| TempProbeFiles { out, err })
    }) {
        Ok(f) => f,
        Err(e) => return (-1, String::new(), format!("oserror: {e}")),
    };
    let out_write = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&files.out)
    {
        Ok(f) => f,
        Err(e) => return (-1, String::new(), format!("oserror: {e}")),
    };
    let err_write = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&files.err)
    {
        Ok(f) => f,
        Err(e) => return (-1, String::new(), format!("oserror: {e}")),
    };

    let mut command = Command::new(cmd[0]);
    command
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(out_write))
        .stderr(Stdio::from(err_write));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0 on POSIX): the probe runs in windowless
        // processes (pythonw gateway / kanban workers) where a console
        // child would otherwise flash a visible window per probe.
        command.creation_flags(0x08000000);
    }

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (-1, String::new(), "not found".to_string())
        }
        Err(e) => return (-1, String::new(), format!("oserror: {e}")),
    };

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (-1, String::new(), "timeout".to_string());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return (-1, String::new(), format!("oserror: {e}"));
            }
        }
    };
    let rc = status.code().unwrap_or(-1);

    let out = read_lossy_stripped(&files.out);
    let err = read_lossy_stripped(&files.err);
    (rc, out, err)
}

fn read_lossy_stripped(path: &PathBuf) -> String {
    let mut buf = Vec::new();
    match fs::File::open(path).and_then(|mut f| f.read_to_end(&mut buf)) {
        Ok(_) => String::from_utf8_lossy(&buf).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// `shutil.which`-equivalent: first executable in PATH, or None.
fn which(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let cand = dir.join(binary);
        if cand.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let exec = fs::metadata(&cand)
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                if exec {
                    return Some(cand);
                }
            }
            #[cfg(windows)]
            {
                let ext = cand
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let exec = matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com");
                if exec {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// Short version string like `3.12.4` for `binary`, or None.
fn _python_version_of(binary: &str) -> Option<String> {
    which(binary)?;
    let (rc, out, _err) = _run(
        &[
            binary,
            "-c",
            "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}')",
        ],
        RUN_TIMEOUT,
    );
    if rc == 0 && !out.is_empty() {
        return Some(out);
    }
    None
}

/// True if `<binary> -m pip --version` succeeds.
fn _has_pip_module(binary: &str) -> bool {
    if which(binary).is_none() {
        return false;
    }
    let (rc, _out, _err) = _run(&[binary, "-m", "pip", "--version"], RUN_TIMEOUT);
    rc == 0
}

/// True when `binary`'s install location is PEP-668 externally managed.
fn _detect_pep668(binary: &str) -> bool {
    if which(binary).is_none() {
        return false;
    }
    let code = concat!(
        "import sys, os;",
        "stdlib = os.path.dirname(os.__file__);",
        "marker = os.path.join(stdlib, 'EXTERNALLY-MANAGED');",
        "print('yes' if os.path.exists(marker) else 'no')"
    );
    let (rc, out, _err) = _run(&[binary, "-c", code], RUN_TIMEOUT);
    rc == 0 && out.trim() == "yes"
}

/// If `pip` is on PATH, return the Python version it's bound to
/// (the parenthesised `3.12` from `pip --version`), or None.
fn _pip_python_version() -> Option<String> {
    which("pip")?;
    let (rc, out, _err) = _run(&["pip", "--version"], RUN_TIMEOUT);
    if rc != 0 || out.is_empty() {
        return None;
    }
    // Parse trailing "(python X.Y)".
    if out.contains("(python ") && out.ends_with(')') {
        if let Some(tail) = out.rsplit_once("(python ") {
            let version = tail.1.strip_suffix(')').unwrap_or(tail.1).trim();
            return Some(version.to_string());
        }
    }
    None
}

/// Build the one-liner. Returns "" when nothing notable is detected.
fn _build_probe_line() -> String {
    if let Some(f) = TEST_BUILD_OVERRIDE.lock().unwrap().clone() {
        return f();
    }

    // Bail out if a remote terminal backend is configured; the host's
    // Python state isn't where the agent's tools run.
    let backend = env::var("TERMINAL_ENV")
        .unwrap_or_else(|_| "local".to_string())
        .trim()
        .to_lowercase();
    if REMOTE_BACKENDS.contains(&backend.as_str()) {
        return String::new();
    }

    let py3_ver = _python_version_of("python3");
    let py_ver = _python_version_of("python"); // systems with a `python` alias
    let py3_has_pip = if py3_ver.is_some() {
        _has_pip_module("python3")
    } else {
        false
    };
    let pip_bound_to = _pip_python_version();
    let py3_pep668 = if py3_ver.is_some() {
        _detect_pep668("python3")
    } else {
        false
    };
    // Bare which() is correct here, unlike Hermes's own uv call sites.
    let has_uv = which("uv").is_some();

    let mismatch = match (&pip_bound_to, &py3_ver) {
        (Some(pip), Some(py3)) => !py3.starts_with(pip.as_str()),
        _ => false,
    };
    let silent_conditions = py3_ver.is_some()
        && py3_has_pip
        && !mismatch
        && (!py3_pep668 || has_uv);
    if silent_conditions {
        return String::new();
    }

    let mut bits: Vec<String> = Vec::new();
    if let Some(py3) = &py3_ver {
        let mut py3_bit = format!("python3={py3}");
        if !py3_has_pip {
            py3_bit.push_str(" (no pip module)");
        }
        bits.push(py3_bit);
    } else {
        bits.push("python3=missing".to_string());
    }

    if let Some(py) = &py_ver {
        if Some(py.as_str()) != py3_ver.as_deref() {
            bits.push(format!("python={py}"));
        }
    } else if py3_ver.is_some() {
        // Common on Debian/Ubuntu — call it out so the model doesn't
        // type `python` and hit "command not found".
        bits.push("python=missing (use python3)".to_string());
    }

    match (&pip_bound_to, py3_has_pip) {
        (Some(pv), _) => {
            if mismatch {
                bits.push(format!("pip→python{pv} (mismatch)"));
            } else if !py3_has_pip {
                bits.push(format!("pip→python{pv}"));
            }
        }
        (None, true) => {
            // `pip` not on PATH but `python3 -m pip` works — nothing to add.
        }
        (None, false) => bits.push("pip=missing".to_string()),
    }

    if py3_pep668 {
        bits.push("PEP 668=yes (use venv or uv)".to_string());
    }
    if has_uv {
        bits.push("uv=installed".to_string());
    }

    if bits.is_empty() {
        return String::new();
    }
    format!("Python toolchain: {}.", bits.join(", "))
}

/// Body of the single probe thread — computes and publishes the line.
fn probe_worker(gen: u64) {
    // Never let probe failure propagate (mirrors upstream try/except).
    let line =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(build_probe_line))
            .unwrap_or_default();
    let mut state = STATE.lock().unwrap();
    if state.probe_gen != gen {
        return; // superseded by a reset (tests) — discard stale result
    }
    state.cached_line = Some(line);
    state.probe_done = true;
    PROBE_COND.notify_all();
}

/// Start the probe worker if it isn't running and hasn't finished.
fn ensure_probe_started() {
    let mut state = STATE.lock().unwrap();
    if state.probe_done || state.probe_thread_started {
        return;
    }
    let gen = state.probe_gen;
    state.probe_thread_started = true;
    thread::Builder::new()
        .name("env-probe".to_string())
        .spawn(move || probe_worker(gen))
        .expect("spawn env-probe thread");
}

/// Return the cached probe line (building it on first call).
///
/// Returns "" when the environment is clean. The probe itself always runs
/// in a single background worker thread; this function waits on its
/// completion for at most `PROBE_WAIT_TIMEOUT` and then fails open with
/// "". A wedged probe subprocess can therefore never block
/// system-prompt construction. `force_refresh` is for tests; real
/// callers should never need it.
pub fn get_environment_probe_line(force_refresh: bool) -> String {
    if force_refresh {
        let mut state = STATE.lock().unwrap();
        state.cached_line = None;
        state.probe_done = false;
        state.probe_thread_started = false;
        state.probe_gen += 1;
        state.wait_already_timed_out = false;
    }

    {
        let state = STATE.lock().unwrap();
        if state.probe_done {
            return state.cached_line.clone().unwrap_or_default();
        }
    }

    ensure_probe_started();

    let mut state = STATE.lock().unwrap();
    let timeout = if state.wait_already_timed_out {
        WAIT_ALREADY_TIMED_OUT_PEEK
    } else {
        probe_wait_timeout()
    };
    let deadline = Instant::now() + timeout;
    loop {
        if state.probe_done {
            return state.cached_line.clone().unwrap_or_default();
        }
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let (guard, wait_result) = PROBE_COND
            .wait_timeout(state, deadline - now)
            .unwrap_or_else(|e| e.into_inner());
        state = guard;
        if wait_result.timed_out() {
            break;
        }
        // Spurious wake: re-check probe_done at loop top.
    }
    if !state.wait_already_timed_out {
        state.wait_already_timed_out = true;
        log::warn!(
            "env_probe did not finish within {:.0}s; building the system prompt without the Python toolchain line",
            probe_wait_timeout().as_secs_f64(),
        );
    }
    String::new()
}

/// Kick off the probe in a background thread so the first system-prompt
/// build doesn't pay the ~0.5s of subprocess calls on the
/// time-to-first-token critical path. Idempotent and fail-safe.
pub fn warm_environment_probe_async() {
    ensure_probe_started();
}

/// Test helper — clear the cache between probe scenarios (mirrors
/// upstream `_reset_cache_for_tests`).
pub fn _reset_cache_for_tests() {
    let mut state = STATE.lock().unwrap();
    state.cached_line = None;
    state.probe_done = false;
    state.probe_thread_started = false;
    state.probe_gen += 1;
    state.wait_already_timed_out = false;
}

/// Test helper — block until the probe worker publishes (mirrors
/// upstream `assert env_probe._PROBE_DONE.wait(timeout=10)`).
pub fn _probe_done_wait_for_tests(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut state = STATE.lock().unwrap();
    loop {
        if state.probe_done {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        let (guard, wait_result) = PROBE_COND
            .wait_timeout(state, deadline - now)
            .unwrap_or_else(|e| e.into_inner());
        state = guard;
        if wait_result.timed_out() {
            return state.probe_done;
        }
    }
}
