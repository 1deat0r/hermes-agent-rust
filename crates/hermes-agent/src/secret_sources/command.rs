//! `command` secret source — resolve secrets via a user-configured helper.
//!
//! PARITY: `agent/secret_sources/command.py` @ 5d59366 (whole module,
//! 383 lines; the
//! `CommandSource` registry adapter included).
//!
//! Ports the security semantics of the desktop app's TypeScript
//! `CommandSecretsProvider` line-for-line where it matters:
//!
//! * The command string is the USER'S OWN configuration (same trust level
//!   as their `.env`), so it runs via `/bin/sh -c`.
//! * The requested key is passed to the child ONLY via the
//!   `HERMES_SECRET_KEY` environment variable — never interpolated into
//!   the shell string, so a hostile key name is inert data, not code.
//! * Hard timeout (default 3s) + output cap (default 1 MiB); any failure
//!   (non-zero exit, timeout, spawn failure, oversized output) degrades to
//!   "no value" rather than raising.
//! * Failures log ONLY structured fields (exit code / signal) — never the
//!   command string, the helper's stderr, or any secret value. The
//!   helper's stderr is captured and DISCARDED.
//! * POSIX-only: on Windows the source degrades to an empty result.
//! * The helper runs exactly ONCE with an empty `HERMES_SECRET_KEY` on the
//!   startup/apply path — never per-key in a loop.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use super::base::{coerce_float, source_child_env, ErrorKind, FetchResult, SecretSource};

/// Hard cap so a hung helper can never wedge startup — a configured helper
/// MUST be fast and NON-INTERACTIVE.
///
/// PARITY: `_COMMAND_TIMEOUT_SECONDS` (upstream line 40).
pub const COMMAND_TIMEOUT_SECONDS: f64 = 3.0;

/// Defensive cap on helper output (1 MiB).
///
/// PARITY: `_MAX_OUTPUT_BYTES` (upstream line 42).
pub const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// PARITY: `_ENV_LINE` (upstream line 43) — `^([A-Za-z_][A-Za-z0-9_]*)=(.*)$`.
static ENV_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$").expect("env line re"));

/// Base64 padding disambiguation: an env-shaped line whose "value" part is
/// empty or all `=` is a bare base64 secret, not a dotenv entry.
///
/// PARITY: the `re.fullmatch(r"=*", ...)` check (upstream lines 197-201).
static PADDING_ONLY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^=*$").expect("padding re"));

fn is_windows() -> bool {
    cfg!(windows)
}

/// Signal number → name for structured failure logs (upstream
/// `Signals(-code).name`, numeric fallback).
fn signal_name(signum: i32) -> String {
    match signum {
        1 => "SIGHUP".to_string(),
        2 => "SIGINT".to_string(),
        3 => "SIGQUIT".to_string(),
        4 => "SIGILL".to_string(),
        6 => "SIGABRT".to_string(),
        8 => "SIGFPE".to_string(),
        9 => "SIGKILL".to_string(),
        11 => "SIGSEGV".to_string(),
        13 => "SIGPIPE".to_string(),
        14 => "SIGALRM".to_string(),
        15 => "SIGTERM".to_string(),
        n => n.to_string(),
    }
}

/// Strip a single layer of matching surrounding quotes from a dotenv
/// value.
///
/// Requires length >= 2 so a lone quote is left intact rather than
/// collapsing to empty, and `""`/`''` correctly yield an empty string.
///
/// PARITY: `unquote_dotenv_value` (upstream lines 55-70).
pub fn unquote_dotenv_value(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Parse a secret-fetch helper's stdout. Supports BOTH shapes:
///
/// * a bare value (single secret): the whole trimmed stdout is the value.
/// * a dotenv blob (KEY=VALUE lines): return the entry for `wanted_key`.
///
/// Mirrors the TS `parseSecretOutput` exactly, including the cross-key
/// misroute guard and the base64-padding disambiguation.
///
/// PARITY: `parse_secret_output` (upstream lines 73-133).
pub fn parse_secret_output(stdout: &str, wanted_key: &str) -> Option<String> {
    let text = stdout.replace("\r\n", "\n");

    // 1. Exact dotenv match wins: deterministic, never another key's value.
    let dotenv_lines: Vec<String> = text
        .lines()
        .map(|raw| raw.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#') && ENV_LINE.is_match(line))
        .collect();
    for line in &dotenv_lines {
        if let Some(caps) = ENV_LINE.captures(line) {
            if &caps[1] == wanted_key {
                let value = unquote_dotenv_value(&caps[2]);
                // Whitespace-only (e.g. a quoted `K="  "` placeholder) is
                // "no value": it would otherwise flow into an Authorization
                // header → guaranteed 401.
                return if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                };
            }
        }
    }

    // 2. A multi-key dotenv dump that does NOT contain the wanted key →
    //    None. Only >=2 env-shaped lines count as a dump: a SINGLE
    //    non-matching env-shaped line falls through to the bare-value
    //    branch (a bare secret can itself match KEY=VALUE — base64 with
    //    '=' padding — and must not be misclassified as a dump).
    if dotenv_lines.len() > 1 {
        return None;
    }

    // 3. Otherwise treat the whole output as a single bare value.
    let value = text.trim();
    if value.is_empty() {
        return None;
    }

    // SECURITY (S2): a single env-shaped line for a DIFFERENT key must not
    // be returned as the wanted secret — cross-provider credential
    // leakage. Disambiguation from a bare base64 secret: base64 padding
    // only ever produces an env-shaped line whose "value" part is empty or
    // all '=' (`dGVzdA==` → key `dGVzdA`, value `=`).
    if let Some(caps) = ENV_LINE.captures(value) {
        if caps[1] != *wanted_key && !PADDING_ONLY_RE.is_match(caps[2].trim()) {
            return None;
        }
    }
    Some(value.to_string())
}

/// Parse a KEY=VALUE blob into a map (the list/enumerate path).
///
/// PARITY: `_parse_dotenv_map` (upstream lines 226-243) — only env-shaped
/// lines contribute; comments and non-matching lines are skipped.
pub fn parse_dotenv_map(stdout: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in stdout.replace("\r\n", "\n").split('\n') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(caps) = ENV_LINE.captures(line) {
            out.push((caps[1].to_string(), unquote_dotenv_value(&caps[2])));
        }
    }
    out
}

/// Run the helper via `/bin/sh -c` and return its stdout, or None.
///
/// The key is passed as DATA via `HERMES_SECRET_KEY` — never interpolated
/// into the command string. Both streams are captured; stderr is
/// DISCARDED. Any failure logs structured fields only (exit code / signal
/// — never the command string or the helper's stderr) and returns None.
///
/// PARITY: `_run_helper` (upstream lines 246-323) — hard timeout kills the
/// whole process group (a helper script may fork children that would
/// otherwise keep the pipe open).
fn run_helper(
    command: &str,
    secret_key: &str,
    timeout_seconds: f64,
    max_output_bytes: usize,
) -> Option<String> {
    if is_windows() {
        eprintln!(
            "[secrets:command] the 'command' provider is POSIX-only (needs /bin/sh); \
             resolving no value on Windows"
        );
        return None;
    }

    let mut spawned = {
        use std::os::unix::process::CommandExt;
        // The helper legitimately gets the caller's env (it may need any
        // credential to resolve the secret) — but a multiplex profile
        // only its own: `source_child_env` returns the per-fetch view
        // when one is installed, and None on the single-profile path
        // where the process env IS the caller's env (upstream
        // `build_subprocess_env` shape, environments surface PENDING).
        let mut child_cmd = Command::new("/bin/sh");
        child_cmd.arg("-c").arg(command);
        match source_child_env() {
            Some(view) => {
                child_cmd.env_clear();
                for (k, v) in &view {
                    child_cmd.env(k, v);
                }
            }
            None => {}
        }
        child_cmd
            .env("HERMES_SECRET_KEY", secret_key)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // captured and DISCARDED — never inherited
            .process_group(0) // so the hard timeout can kill the whole group
            .spawn()
    };

    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "[secrets:command] helper failed to spawn; resolving no value: errno={}",
                e.raw_os_error().unwrap_or(-1)
            );
            return None;
        }
    };
    let pid = child.id() as i32;

    // Drain stdout and stderr on SEPARATE buffers (upstream pipes them
    // separately and discards stderr): merging them would pollute the
    // parsed secrets with helper diagnostics.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let drain = std::thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut out);
        }
        let mut err = Vec::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut err);
        }
        (out, err)
    });

    let deadline = Instant::now() + Duration::from_secs_f64(timeout_seconds);
    let mut timed_out = false;
    let mut exit_code: Option<i32> = None;
    while exit_code.is_none() {
        match child.try_wait() {
            Ok(Some(status)) => exit_code = Some(status.code().unwrap_or(-1)),
            Ok(None) => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                eprintln!(
                    "[secrets:command] helper failed; resolving no value: code=? signal=none"
                );
                let _ = drain.join();
                return None;
            }
        }
    }

    if timed_out {
        // Kill the whole process group.
        let timeout_text = if timeout_seconds.fract() == 0.0 {
            format!("{:.0}", timeout_seconds)
        } else {
            format!("{timeout_seconds}")
        };
        unsafe {
            libc::killpg(libc::getpgid(pid), libc::SIGKILL);
        }
        let _ = drain.join();
        let _ = child.wait();
        eprintln!("[secrets:command] helper timed out after {timeout_text}s; resolving no value");
        return None;
    }

    let code = exit_code.unwrap_or(-1);
    if code != 0 {
        // Structured fields ONLY — never the command string or the
        // helper's stderr (either can carry secret material). Negative
        // codes are signals; resolve the name like upstream's
        // `Signals(-code).name` (numeric fallback when unknown).
        if code < 0 {
            eprintln!(
                "[secrets:command] helper failed; resolving no value: code=? signal={}",
                signal_name(-code)
            );
        } else {
            eprintln!(
                "[secrets:command] helper failed; resolving no value: code={code} signal=none"
            );
        }
        let _ = drain.join();
        return None;
    }

    let (stdout_bytes, _stderr_discarded) = drain.join().unwrap_or_default();
    if stdout_bytes.len() > max_output_bytes {
        eprintln!(
            "[secrets:command] helper output exceeded the {max_output_bytes}-byte cap; \
             resolving no value"
        );
        return None;
    }
    Some(String::from_utf8_lossy(&stdout_bytes).into_owned())
}

/// Parse a KEY=VALUE blob into a map (public alias of the internal parser).
pub fn parse_dotenv_map_public(stdout: &str) -> Vec<(String, String)> {
    parse_dotenv_map(stdout)
}

/// Resolve a single secret by running the helper with the key in
/// `HERMES_SECRET_KEY`. Returns None on any failure — never raises.
///
/// PARITY: `get_command_secret` (upstream lines 320-331).
pub fn get_command_secret(
    command: &str,
    key: &str,
    timeout_seconds: f64,
    max_output_bytes: usize,
) -> Option<String> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let stdout = run_helper(command, key, timeout_seconds, max_output_bytes)?;
    parse_secret_output(&stdout, key)
}

/// Enumerate secrets by running the helper ONCE with an empty key.
///
/// Returns the dotenv map ONLY when the helper emits a KEY=VALUE blob; a
/// bare-value helper returns an empty map. Never raises.
///
/// PARITY: `list_command_secrets` (upstream lines 334-346).
pub fn list_command_secrets(
    command: &str,
    timeout_seconds: f64,
    max_output_bytes: usize,
) -> Vec<(String, String)> {
    let command = command.trim();
    if command.is_empty() {
        return Vec::new();
    }
    let Some(stdout) = run_helper(command, "", timeout_seconds, max_output_bytes) else {
        return Vec::new();
    };
    parse_dotenv_map(&stdout)
}

/// Run the helper once at startup and apply its KEY=VALUE output.
///
/// LEGACY shim retained for API symmetry; the startup path goes
/// through `CommandSource` + the registry orchestrator instead (which
/// owns precedence and the environ writes).
///
/// PARITY: `apply_command_secrets` (upstream lines 200-265). Writes to
/// the process env — scoped to this explicit entry point like the
/// onepassword equivalent.
pub fn apply_command_secrets(
    command: &str,
    override_existing: bool,
    timeout_seconds: f64,
    max_output_bytes: usize,
) -> FetchResult {
    let mut result = FetchResult::default();
    let command = command.trim();
    if command.is_empty() {
        result.error = Some(
            "secrets.command.enabled is true but secrets.command.command is empty. \
             Set the helper command in config.yaml."
                .to_string(),
        );
        return result;
    }
    if is_windows() {
        result.warnings.push(
            "the 'command' secret source is POSIX-only (needs /bin/sh); skipping on Windows"
                .to_string(),
        );
        return result;
    }
    // The list/enumerate path: run the helper exactly ONCE with an
    // empty HERMES_SECRET_KEY and parse its stdout as a dotenv blob.
    let Some(stdout) = run_helper(command, "", timeout_seconds, max_output_bytes) else {
        // run_helper already logged structured fields.
        result.warnings.push(
            "helper command failed at startup; no secrets applied (process env / .env values remain in effect)"
                .to_string(),
        );
        return result;
    };
    let secrets = parse_dotenv_map(&stdout);
    if secrets.is_empty() {
        result.warnings.push(
            "helper output was not a KEY=VALUE map; nothing applied at startup (a bare-value helper still resolves single keys on demand)"
                .to_string(),
        );
        return result;
    }
    for (key, value) in secrets {
        if value.trim().is_empty() {
            // Whitespace-only placeholders are "no value" — applying them
            // would flow into an Authorization header → guaranteed 401.
            result.skipped.push(key);
            continue;
        }
        if !override_existing && std::env::var(&key).map(|v| !v.is_empty()).unwrap_or(false) {
            // Process env / .env win — same precedence as bitwarden.
            result.skipped.push(key);
            continue;
        }
        // SAFETY: explicit sync entry point only (see docstring).
        unsafe { std::env::set_var(&key, &value) };
        result.secrets.insert(key.clone(), value);
        result.applied.push(key);
    }
    result
}

/// The registry-facing `SecretSource` adapter (bulk shape: the helper
/// enumerates a KEY=VALUE blob in one run).
///
/// PARITY: `CommandSource` (upstream lines 370-501).
pub struct CommandSource;

impl SecretSource for CommandSource {
    fn name(&self) -> &str {
        "command"
    }
    fn label(&self) -> &str {
        "Command helper"
    }
    fn shape(&self) -> &str {
        "bulk"
    }

    fn config_schema(&self) -> Value {
        serde_json::json!({
            "enabled": {"description": "Master switch", "default": false},
            "command": {
                "description": "Helper run via /bin/sh -c; must print a KEY=VALUE blob on stdout",
                "default": "",
            },
            "helper_timeout_seconds": {
                "description": "Hard timeout for one helper run",
                "default": COMMAND_TIMEOUT_SECONDS,
            },
            "override_existing": {
                "description": "Helper values overwrite .env/shell values",
                "default": false,
            },
        })
    }

    fn fetch(&self, cfg: &Value, _home_path: &Path) -> FetchResult {
        let mut result = FetchResult::default();
        let Some(map) = cfg.as_object() else {
            result.error = Some(
                "secrets.command.enabled is true but secrets.command.command is empty. \
                 Set the helper command in config.yaml."
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::NotConfigured);
            return result;
        };

        let command = map
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if command.is_empty() {
            result.error = Some(
                "secrets.command.enabled is true but secrets.command.command is empty. \
                 Set the helper command in config.yaml."
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::NotConfigured);
            return result;
        }

        if is_windows() {
            result.error = Some(
                "the 'command' secret source is POSIX-only (needs /bin/sh); skipping on Windows"
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::NotConfigured);
            return result;
        }

        let timeout = coerce_float(map.get("helper_timeout_seconds"), COMMAND_TIMEOUT_SECONDS);
        // NOTE: upstream passes coerce_float straight through (a negative
        // config would raise in communicate()); here non-positive budgets
        // fall back to the default — a stuck startup is worse than a
        // loud config.
        let timeout = if timeout > 0.0 {
            timeout
        } else {
            COMMAND_TIMEOUT_SECONDS
        };

        let Some(stdout) = run_helper(&command, "", timeout, MAX_OUTPUT_BYTES) else {
            result.error = Some(
                "helper command failed (see structured fields above); no secrets applied"
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::Internal);
            return result;
        };

        let secrets = parse_dotenv_map(&stdout);
        if secrets.is_empty() {
            result
                .warnings
                .push("helper output was not a KEY=VALUE map; nothing to apply".to_string());
            return result;
        }
        result.secrets = secrets.into_iter().collect();
        result
    }

    fn remediation_hints(&self) -> std::collections::HashMap<ErrorKind, String> {
        std::collections::HashMap::from([
            (
                ErrorKind::NotConfigured,
                "Set secrets.command.command in config.yaml to a fast, non-interactive \
                 helper that prints KEY=VALUE lines."
                    .to_string(),
            ),
            (
                ErrorKind::Internal,
                "Run the helper manually in a shell to see its real error — Hermes \
                 discards helper stderr so diagnostics can't leak secret material."
                    .to_string(),
            ),
        ])
    }
}
