//! Verification runner: execute a Recipe's phases and smoke-test the app.
//!
//! PARITY: `agent/verify/runner.py` @ b9aa928 (whole module). Scoped port
//! of the execution flow grok-cli's verify sub-agent performs
//! (install/bootstrap → build → test → start in background → readiness
//! loop → teardown), reimplemented as a plain subprocess runner.
//!
//! Commands come from the project's own recipe (its package.json scripts,
//! Makefile targets, etc.) and are executed through the shell on purpose:
//! this is a developer tool running the project's own build commands in
//! the project's own checkout — the same trust level as the terminal tool.
//!
//! TRANSLATION NOTES: `shell=True` becomes `sh -c`; readiness polling uses
//! a raw HTTP/1.0 GET over `TcpStream` (any response — even 4xx/5xx —
//! proves the server is up, matching upstream's HTTPError arm); the child
//! is spawned in its own process group (`start_new_session`) so teardown
//! can signal the whole tree.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::json;

use super::recipes::Recipe;

/// PARITY: `DEFAULT_PHASE_TIMEOUT` (upstream line 17).
pub const DEFAULT_PHASE_TIMEOUT: f64 = 600.0;
/// PARITY: `DEFAULT_READY_TIMEOUT` (upstream line 18).
pub const DEFAULT_READY_TIMEOUT: f64 = 60.0;
/// PARITY: `_TAIL_CHARS` (upstream line 19).
const TAIL_CHARS: usize = 2000;
/// PARITY: `PHASE_ORDER` (upstream line 20).
pub const PHASE_ORDER: [&str; 3] = ["bootstrap", "build", "test"];

/// PARITY: `PhaseResult` (upstream lines 23-45).
#[derive(Debug, Clone)]
pub struct PhaseResult {
    pub phase: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration: f64,
    pub output_tail: String,
    pub timed_out: bool,
}

impl PhaseResult {
    /// PARITY: the `ok` property — exit 0 and no timeout.
    pub fn ok(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out
    }

    pub fn to_dict(&self) -> serde_json::Value {
        json!({
            "phase": self.phase,
            "command": self.command,
            "exitCode": self.exit_code,
            "duration": (self.duration * 1000.0).round() / 1000.0,
            "ok": self.ok(),
            "timedOut": self.timed_out,
            "outputTail": self.output_tail,
        })
    }
}

/// PARITY: `ReadinessResult` (upstream lines 48-67).
#[derive(Debug, Clone)]
pub struct ReadinessResult {
    pub url: String,
    pub ready: bool,
    pub status_code: Option<i64>,
    pub duration: f64,
    pub error: Option<String>,
    pub output_tail: String,
}

impl ReadinessResult {
    pub fn to_dict(&self) -> serde_json::Value {
        json!({
            "url": self.url,
            "ready": self.ready,
            "statusCode": self.status_code,
            "duration": (self.duration * 1000.0).round() / 1000.0,
            "error": self.error,
            "outputTail": self.output_tail,
        })
    }
}

/// PARITY: `VerifyResult` (upstream lines 70-93).
#[derive(Debug, Clone, Default)]
pub struct VerifyResult {
    pub recipe_name: String,
    pub phases: Vec<PhaseResult>,
    pub readiness: Option<ReadinessResult>,
}

impl VerifyResult {
    /// PARITY: the `ok` property — every phase ok, and readiness ok when
    /// present.
    pub fn ok(&self) -> bool {
        let phases_ok = self.phases.iter().all(|p| p.ok());
        let readiness_ok = self.readiness.as_ref().map(|r| r.ready).unwrap_or(true);
        phases_ok && readiness_ok
    }

    pub fn to_dict(&self) -> serde_json::Value {
        json!({
            "recipe": self.recipe_name,
            "ok": self.ok(),
            "phases": self.phases.iter().map(|p| p.to_dict()).collect::<Vec<_>>(),
            "readiness": self.readiness.as_ref().map(|r| r.to_dict()),
        })
    }
}

/// PARITY: `_tail` (upstream lines 96-98).
fn tail(text: &str) -> String {
    if text.len() > TAIL_CHARS {
        // Python slices bytes but the payloads are text; character slicing
        // keeps multi-byte output intact.
        let skip = text
            .char_indices()
            .nth(text.chars().count() - TAIL_CHARS)
            .map(|(i, _)| i)
            .unwrap_or(0);
        text[skip..].to_string()
    } else {
        text.to_string()
    }
}

/// PARITY: `_run_phase_command` (upstream lines 101-135) — shell execute in
/// the project root, stdout+stderr merged, bounded by `timeout`; a timeout
/// kills the child and records `timed_out = true` with a `None` exit code.
/// Fold a finished child into a `PhaseResult`.
fn finalize(
    phase: &str,
    command: &str,
    started: Instant,
    output: String,
    exit_code: Option<i32>,
    timed_out: bool,
    on_output: Option<&dyn Fn(&str)>,
) -> PhaseResult {
    let duration = started.elapsed().as_secs_f64();
    if let Some(on_output) = on_output {
        if !output.is_empty() {
            on_output(&output);
        }
    }
    PhaseResult {
        phase: phase.to_string(),
        command: command.to_string(),
        exit_code,
        duration,
        output_tail: tail(&output),
        timed_out,
    }
}

fn run_phase_command(
    phase: &str,
    command: &str,
    root: &Path,
    timeout: f64,
    on_output: Option<&dyn Fn(&str)>,
) -> PhaseResult {
    let started = Instant::now();

    enum Outcome {
        Done(i32, String),
        TimedOut(String),
        Failed,
    }

    let outcome: Outcome = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Err(_) => Outcome::Failed,
        Ok(mut child) => {
            // Drain the merged pipes on a thread so a chatty command cannot
            // deadlock on a full pipe buffer.
            let mut stdout_pipe = child.stdout.take();
            let stderr_pipe = child.stderr.take();
            let reader = std::thread::spawn(move || {
                let mut buffer = Vec::new();
                if let Some(stdout) = stdout_pipe.as_mut() {
                    let _ = stdout.read_to_end(&mut buffer);
                }
                if let Some(mut stderr) = stderr_pipe {
                    let _ = stderr.read_to_end(&mut buffer);
                }
                String::from_utf8_lossy(&buffer).into_owned()
            });
            let deadline = Instant::now() + Duration::from_secs_f64(timeout);
            let mut timed_out = false;
            let code: Option<i32> = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Some(status.code().unwrap_or(-1)),
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            let _ = child.kill();
                            let _ = child.wait();
                            timed_out = true;
                            break None;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => break None,
                }
            };
            let output = reader.join().unwrap_or_default();
            if timed_out {
                Outcome::TimedOut(output)
            } else {
                match code {
                    Some(code) => Outcome::Done(code, output),
                    None => Outcome::Failed,
                }
            }
        }
    };

    match outcome {
        Outcome::Done(exit_code, output) => finalize(
            phase,
            command,
            started,
            output,
            Some(exit_code),
            false,
            on_output,
        ),
        Outcome::TimedOut(output) => {
            finalize(phase, command, started, output, None, true, on_output)
        }
        Outcome::Failed => finalize(
            phase,
            command,
            started,
            String::new(),
            None,
            false,
            on_output,
        ),
    }
}

/// One readiness probe over a raw HTTP/1.0 GET.
///
/// PARITY: the `urllib.request.urlopen` arm of `_poll_readiness` — *any*
/// HTTP response (even 4xx/5xx) proves the server is up and yields its
/// status code; connection failures yield the error string.
fn probe_once(url: &str, timeout: f64) -> Result<i64, String> {
    // url = http://127.0.0.1:<port><path> — parse host/port + path.
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "unsupported URL scheme".to_string())?;
    let (authority, path) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };
    let addr = if let Some(colon) = authority.rfind(':') {
        match authority[colon + 1..].parse::<u16>() {
            Ok(port) => (authority[..colon].to_string(), port),
            Err(e) => return Err(format!("{e}")),
        }
    } else {
        (authority.to_string(), 80)
    };
    let socket = format!("{}:{}", addr.0, addr.1);
    let socket_addr = match socket.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => return Err("could not resolve address".to_string()),
        },
        Err(e) => return Err(e.to_string()),
    };
    let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs_f64(timeout))
        .map_err(|e| e.to_string())?;
    let mut stream = stream;
    stream
        .set_read_timeout(Some(Duration::from_secs_f64(timeout)))
        .map_err(|e| e.to_string())?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: {authority}\r\n\r\n")
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    let head = String::from_utf8_lossy(&response);
    let status_line = head.lines().next().ok_or("empty response")?;
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<i64>().ok())
        .ok_or_else(|| "unparseable status line".to_string())
}

/// PARITY: `_poll_readiness` (upstream lines 138-152) — poll until the
/// deadline at 1s intervals; any HTTP status means up.
fn poll_readiness(url: &str, timeout: f64, interval: f64) -> (bool, Option<i64>, Option<String>) {
    let deadline = Instant::now() + Duration::from_secs_f64(timeout);
    let mut last_error: Option<String> = None;
    while Instant::now() < deadline {
        match probe_once(url, 5.0) {
            Ok(status) => return (true, Some(status), None),
            Err(err) => last_error = Some(err),
        }
        std::thread::sleep(Duration::from_secs_f64(interval));
    }
    (false, None, last_error)
}

/// PARITY: `_terminate_process_group` (upstream lines 155-192) — SIGTERM
/// the child's whole process group, wait 10s, then SIGKILL stragglers.
fn terminate_process_group(child_pid: i32) {
    if unsafe { libc::waitpid(child_pid, std::ptr::null_mut(), libc::WNOHANG) } != 0 {
        return; // already exited
    }
    let pgid = unsafe { libc::getpgid(child_pid) };
    if pgid < 0 {
        return;
    }
    if unsafe { libc::killpg(pgid, libc::SIGTERM) } != 0 {
        return; // ProcessLookupError / PermissionError
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let rc = unsafe { libc::waitpid(child_pid, std::ptr::null_mut(), libc::WNOHANG) };
        if rc == child_pid {
            return;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    unsafe { libc::killpg(pgid, libc::SIGKILL) };
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let rc = unsafe { libc::waitpid(child_pid, std::ptr::null_mut(), libc::WNOHANG) };
        if rc == child_pid || rc < 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// PARITY: `_run_start_phase` (upstream lines 195-236) — launch the start
/// command in its own process group, poll the readiness URL, then tear the
/// group down and collect the output tail.
fn run_start_phase(
    recipe: &Recipe,
    root: &Path,
    ready_timeout: f64,
    port_override: Option<i64>,
) -> ReadinessResult {
    let start = recipe
        .start
        .as_deref()
        .expect("start phase requires a start command");
    let port = port_override.or(recipe.port).unwrap_or(8000);
    let url = format!("http://127.0.0.1:{port}{}", recipe.readiness_path);
    let started = Instant::now();
    // `start_new_session=True` — the child's own process group must be
    // created AT SPAWN TIME (a post-spawn setpgid races the exec and can
    // leave the child in our group, where a killpg would signal us).
    let spawned = Command::new("sh")
        .arg("-c")
        .arg(start)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn();
    let (child_pid, mut stdout_pipe) = match spawned {
        Ok(mut child) => {
            let pid = child.id() as i32;
            let stdout_pipe = child.stdout.take();
            (pid, stdout_pipe)
        }
        Err(e) => (-(e.raw_os_error().unwrap_or(-1)), None),
    };
    if child_pid <= 0 {
        return ReadinessResult {
            url,
            ready: false,
            status_code: None,
            duration: started.elapsed().as_secs_f64(),
            error: Some(format!("failed to spawn (os error {})", -child_pid)),
            output_tail: String::new(),
        };
    }

    let (ready, status, error) = poll_readiness(&url, ready_timeout, 1.0);
    terminate_process_group(child_pid);
    let output = stdout_pipe
        .as_mut()
        .map(|pipe| {
            let mut buffer = Vec::new();
            let _ = pipe.read_to_end(&mut buffer);
            String::from_utf8_lossy(&buffer).into_owned()
        })
        .unwrap_or_default();
    let _ = spawned;
    ReadinessResult {
        url,
        ready,
        status_code: status,
        duration: started.elapsed().as_secs_f64(),
        error,
        output_tail: tail(&output),
    }
}

/// Run a verify pass for `recipe` at project `root`.
///
/// Executes the selected command phases sequentially, then (unless
/// `skip_start` or a phase failed) launches `recipe.start` in the
/// background, polls the readiness URL, and tears the process group down.
///
/// PARITY: `run_verify` (upstream lines 239-277).
#[allow(clippy::too_many_arguments)]
pub fn run_verify(
    root: &Path,
    recipe: &Recipe,
    phases: Option<&[&str]>,
    phase_timeout: f64,
    ready_timeout: f64,
    skip_start: bool,
    port_override: Option<i64>,
    stop_on_failure: bool,
    on_output: Option<&dyn Fn(&str)>,
) -> VerifyResult {
    let selected: Vec<&str> = match phases {
        Some(phases) => phases.to_vec(),
        None => {
            let mut all: Vec<&str> = PHASE_ORDER.to_vec();
            all.push("start");
            all
        }
    };
    let mut result = VerifyResult {
        recipe_name: recipe.name.clone(),
        ..VerifyResult::default()
    };

    let mut failed = false;
    for phase in PHASE_ORDER {
        if !selected.contains(&phase) {
            continue;
        }
        let commands: &[String] = match phase {
            "bootstrap" => &recipe.bootstrap,
            "build" => &recipe.build,
            _ => &recipe.test,
        };
        for command in commands {
            let phase_result = run_phase_command(phase, command, root, phase_timeout, on_output);
            if !phase_result.ok() {
                failed = true;
                result.phases.push(phase_result);
                if stop_on_failure {
                    return result;
                }
                continue;
            }
            result.phases.push(phase_result);
        }
    }

    if skip_start || !selected.contains(&"start") || failed || recipe.start.is_none() {
        return result;
    }

    result.readiness = Some(run_start_phase(recipe, root, ready_timeout, port_override));
    result
}

// Silence an unused-path helper kept for documentation parity.
#[allow(unused)]
fn _root_type(_p: PathBuf) -> PathBuf {
    _p
}
