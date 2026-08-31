//! External secret source integrations — the shared contract.
//!
//! PARITY: `agent/secret_sources/base.py` @ b9aa928 (whole module).
//!
//! A secret source is anything that can supply environment-variable-shaped
//! credentials at process startup, after `~/.hermes/.env` has loaded. The
//! contract every source implements is [`SecretSource`]; the orchestrator
//! that runs the enabled sources (`agent/secret_sources/registry.py`,
//! PENDING) implements ordering, mapped-beats-bulk precedence,
//! first-claim-wins conflicts, `override_existing` semantics, and
//! provenance.
//!
//! Shared helpers (`is_valid_env_name`, `scrub_ansi`, `run_secret_cli`)
//! exist so backends don't hand-roll the security-sensitive bits.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

/// Bump ONLY for breaking changes to the required contract surface.
/// Additive optional hooks must ship with defaults and must NOT bump this.
///
/// PARITY: `SECRET_SOURCE_API_VERSION` (upstream line 22).
pub const SECRET_SOURCE_API_VERSION: i64 = 1;

// ── per-fetch environment view ───────────────────────────────────────────

thread_local! {
    static SOURCE_ENVIRONMENT: std::cell::RefCell<Option<HashMap<String, String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Install a per-fetch environment view without changing the process
/// environment. Returns a token the caller must pass to
/// [`reset_source_environment`].
///
/// PARITY: `set_source_environment` (upstream lines 27-29).
pub fn set_source_environment(environ: HashMap<String, String>) -> SourceEnvToken {
    SOURCE_ENVIRONMENT.with(|slot| {
        let previous = slot.borrow_mut().replace(environ);
        SourceEnvToken { previous }
    })
}

/// PARITY: `reset_source_environment` (upstream lines 31-32).
pub fn reset_source_environment(token: SourceEnvToken) {
    SOURCE_ENVIRONMENT.with(|slot| *slot.borrow_mut() = token.previous);
}

/// Opaque reset token carrying the prior environment view.
#[derive(Default)]
pub struct SourceEnvToken {
    previous: Option<HashMap<String, String>>,
}

/// Snapshot the active per-fetch environment (or the process environment
/// when no view is installed).
pub fn get_source_environment_snapshot() -> HashMap<String, String> {
    SOURCE_ENVIRONMENT.with(|slot| {
        slot.borrow()
            .clone()
            .unwrap_or_else(|| std::env::vars().collect())
    })
}

/// Look up one variable through the active per-fetch environment, or the
/// process environment when no view is installed.
///
/// PARITY: `get_source_environment` (upstream lines 35-39) collapsed to
/// per-variable lookup (the Rust seam never needs the whole mapping).
pub fn get_source_env_var(key: &str) -> Option<String> {
    let from_view =
        SOURCE_ENVIRONMENT.with(|slot| slot.borrow().as_ref().and_then(|m| m.get(key).cloned()));
    from_view.or_else(|| std::env::var(key).ok())
}

// ── timeouts ─────────────────────────────────────────────────────────────

/// Generous because a first run may include a one-time CLI binary
/// auto-install (e.g. bws download+verify).
///
/// PARITY: `DEFAULT_FETCH_TIMEOUT_SECONDS` (upstream line 43).
pub const DEFAULT_FETCH_TIMEOUT_SECONDS: f64 = 120.0;

/// Default timeout for run_secret_cli() subprocess invocations.
///
/// PARITY: `DEFAULT_CLI_TIMEOUT_SECONDS` (upstream line 46).
pub const DEFAULT_CLI_TIMEOUT_SECONDS: f64 = 30.0;

// ── failure taxonomy ─────────────────────────────────────────────────────

/// Machine-readable failure taxonomy for `FetchResult.error_kind`.
///
/// A fixed vocabulary keeps startup warnings and `hermes secrets status`
/// uniform across backends, and lets the orchestrator implement
/// kind-dependent policy (e.g. a future stale-cache fallback on
/// NETWORK/TIMEOUT but not on AUTH_FAILED) exactly once.
///
/// PARITY: `ErrorKind` (upstream lines 49-67).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotConfigured,
    BinaryMissing,
    AuthFailed,
    AuthExpired,
    RefInvalid,
    Network,
    EmptyValue,
    Timeout,
    Internal,
}

impl ErrorKind {
    /// The literal `str(ErrorKind.X)` value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::BinaryMissing => "binary_missing",
            Self::AuthFailed => "auth_failed",
            Self::AuthExpired => "auth_expired",
            Self::RefInvalid => "ref_invalid",
            Self::Network => "network",
            Self::EmptyValue => "empty_value",
            Self::Timeout => "timeout",
            Self::Internal => "internal",
        }
    }
}

/// Outcome of one source's fetch.
///
/// `secrets` holds what the source *would* contribute; whether each var is
/// actually applied is the orchestrator's decision. `applied`/`skipped`
/// exist for backward compatibility with the original Bitwarden
/// fetch-and-apply entry point and are left empty by conforming sources.
///
/// PARITY: `FetchResult` (upstream lines 70-92).
#[derive(Debug, Clone, Default)]
pub struct FetchResult {
    pub secrets: HashMap<String, String>,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub error_kind: Option<ErrorKind>,
    /// Path of the helper binary used, when the source is CLI-driven.
    pub binary_path: Option<PathBuf>,
}

impl FetchResult {
    /// PARITY: the `ok` property (upstream lines 93-95).
    pub fn ok(&self) -> bool {
        self.error.is_none()
    }
}

/// One external secret backend.
///
/// Subtypes set the attributes and implement `fetch`; everything else has
/// a sensible default. `fetch` MUST NOT raise or prompt: the config
/// section may be malformed, so treat every field defensively.
///
/// PARITY: `SecretSource` ABC (upstream lines 98-244) — the class
/// attributes become trait methods with the documented defaults, and the
/// remediation kind→string mapping is preserved verbatim.
pub trait SecretSource: Send + Sync {
    /// Config-section key under `secrets:` in config.yaml; lowercase
    /// `[a-z0-9_]+`; also the provenance label for every var supplied.
    fn name(&self) -> &str;
    /// Human-readable name used in startup messages and
    /// `hermes secrets status`.
    fn label(&self) -> &str;
    /// `"mapped"` when the user explicitly binds env-var names to refs;
    /// `"bulk"` when the backend injects whole projects/folders. The
    /// orchestrator gives mapped sources precedence over bulk sources.
    fn shape(&self) -> &str {
        "mapped"
    }
    /// Optional URI scheme this source owns for secret references
    /// (`"op"` for `op://...`); must be unique across registered sources.
    fn scheme(&self) -> Option<&str> {
        None
    }
    /// Contract version this source was built against.
    fn api_version(&self) -> i64 {
        SECRET_SOURCE_API_VERSION
    }

    /// Resolve this source's secrets. MUST NOT raise or prompt.
    fn fetch(&self, cfg: &Value, home_path: &Path) -> FetchResult;

    /// Whether the user turned this source on.
    fn is_enabled(&self, cfg: &Value) -> bool {
        cfg.get("enabled").and_then(Value::as_bool).unwrap_or(false)
    }

    /// May this source overwrite vars that .env / the shell already set?
    /// NEVER extends to vars claimed by another secret source in the same
    /// startup pass.
    fn override_existing(&self, cfg: &Value) -> bool {
        cfg.get("override_existing")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Env vars the orchestrator must never let ANY source overwrite —
    /// typically the source's own bootstrap-auth var (e.g.
    /// `BWS_ACCESS_TOKEN`) so a vault containing its own access token
    /// can't clobber the credential used to reach it.
    fn protected_env_vars(&self) -> Vec<String> {
        Vec::new()
    }

    /// Wall-clock budget the orchestrator enforces around fetch().
    fn fetch_timeout_seconds(&self, cfg: &Value) -> f64 {
        let val = cfg
            .get("timeout_seconds")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| cfg.get("timeout_seconds").and_then(Value::as_f64));
        match val {
            Some(v) if v > 0.0 => v,
            _ => DEFAULT_FETCH_TIMEOUT_SECONDS,
        }
    }

    /// Optional description of this source's config keys (informational,
    /// used by setup surfaces).
    fn config_schema(&self) -> Value {
        json_serde_empty_object()
    }

    /// One-line, actionable next step for a failed fetch — pure
    /// kind→string mapping, never raises, no I/O. Return `None` to
    /// suppress the hint.
    fn remediation(&self, kind: Option<ErrorKind>) -> Option<String> {
        let name = self.name();
        let generic = match kind? {
            ErrorKind::NotConfigured => {
                format!("Run `hermes secrets {name} setup` to finish configuration.")
            }
            ErrorKind::BinaryMissing => {
                format!("Run `hermes secrets {name} setup` to install the helper CLI.")
            }
            ErrorKind::AuthFailed => format!(
                "Credentials rejected — run `hermes secrets {name} setup` to re-authenticate."
            ),
            ErrorKind::AuthExpired => format!(
                "Credentials expired — run `hermes secrets {name} setup` to re-authenticate."
            ),
            ErrorKind::Network => {
                "Network problem reaching the secrets backend — check connectivity and retry."
                    .to_string()
            }
            ErrorKind::Timeout => {
                format!("Backend was slow — raise secrets.{name}.timeout_seconds if this recurs.")
            }
            ErrorKind::RefInvalid | ErrorKind::EmptyValue | ErrorKind::Internal => return None,
        };
        Some(generic)
    }
}

fn json_serde_empty_object() -> Value {
    serde_json::Map::new().into()
}

// ── shared helpers ───────────────────────────────────────────────────────

/// PARITY: `_ENV_NAME_RE` (upstream line 251).
static ENV_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("env name re"));

/// PARITY: `_ANSI_RE` (upstream lines 255-258) — CSI/OSC sequences with the
/// optional terminator, so *unterminated* OSC sequences (a CLI killed
/// mid-write) are also stripped. Intentionally NOT the ansi_strip helper,
/// which is not a superset of this regex.
static ANSI_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\x1b(?:\[[0-9;?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)?)").expect("ansi re")
});

/// True when `name` is a legal environment-variable name.
///
/// PARITY: `is_valid_env_name` (upstream lines 261-263).
pub fn is_valid_env_name(name: &str) -> bool {
    !name.is_empty() && ENV_NAME_RE.is_match(name)
}

/// Strip ANSI escape sequences (whole CSI/OSC sequences, not just ESC).
///
/// PARITY: `scrub_ansi` (upstream lines 266-269) — `text or ""` falsiness.
pub fn scrub_ansi(text: Option<&str>) -> String {
    let text = text.unwrap_or("");
    ANSI_RE.replace_all(text, "").into_owned()
}

/// The completed result of a helper-CLI invocation.
#[derive(Debug, Clone, Default)]
pub struct SecretCliResult {
    pub stdout: String,
    pub stderr: String,
    pub returncode: i32,
}

/// Run a secret-manager helper CLI with a minimal, allowlisted env.
///
/// Security posture shared by every subprocess-driven backend:
/// * argv list only — never a shell.
/// * The child gets PATH/HOME/locale basics plus only the env vars named
///   in `allow_env` (auth/session vars) and `extra_env` — never a copy of
///   the full post-dotenv process environment, which by this point holds
///   every credential Hermes knows about.
/// * `NO_COLOR=1` is set and stderr is ANSI-scrubbed so helper
///   diagnostics can't smuggle escape sequences into Hermes output.
/// * stdin is /dev/null so a helper that decides to prompt fails fast
///   instead of hanging startup.
///
/// Errors on spawn failure or timeout with a message safe to surface;
/// callers own returncode interpretation.
///
/// PARITY: `run_secret_cli` (upstream lines 272-334).
pub fn run_secret_cli(
    argv: &[String],
    allow_env: &[String],
    extra_env: Option<&HashMap<String, String>>,
    timeout: f64,
) -> Result<SecretCliResult, String> {
    const BASE_KEEP: [&str; 10] = [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "TMPDIR",
        "TEMP",
        "LANG",
        "LC_ALL",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ];
    let mut env: HashMap<String, String> = HashMap::new();
    let allow: Vec<&str> = allow_env.iter().map(|s| s.as_str()).collect();
    for key in BASE_KEEP.into_iter().chain(allow) {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }
    if let Some(extra) = extra_env {
        for (k, v) in extra {
            env.insert(k.clone(), v.clone());
        }
    }
    env.entry("NO_COLOR".to_string())
        .or_insert_with(|| "1".to_string());

    let Some(program) = argv.first() else {
        return Err("failed to invoke <empty argv>".to_string());
    };
    let program_name = Path::new(program)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.clone());

    let output = CommandTimeout::run(argv, &env, timeout).map_err(|kind| match kind {
        SpawnError::TimedOut => format!("{program_name} timed out after {timeout:.0}s"),
        SpawnError::Other(e) => format!("failed to invoke {program_name}: {e}"),
    })?;

    Ok(SecretCliResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        // stderr/stdout are ANSI-scrubbed so helper diagnostics can't
        // smuggle escape sequences into Hermes output.
        stderr: scrub_ansi(Some(&String::from_utf8_lossy(&output.stderr))),
        returncode: output.status,
    })
}

enum SpawnError {
    TimedOut,
    Other(String),
}

struct CommandTimeout {
    child: std::process::Child,
}

impl CommandTimeout {
    fn run(
        argv: &[String],
        env: &HashMap<String, String>,
        timeout: f64,
    ) -> Result<CommandOutput, SpawnError> {
        use std::io::Read;
        use std::process::{Command, Stdio};

        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.env_clear();
        for (k, v) in env {
            command.env(k, v);
        }
        let mut child = command
            .spawn()
            .map_err(|e| SpawnError::Other(e.to_string()))?;
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        // Drain pipes on threads so chatty helpers cannot deadlock.
        let out_t = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(o) = stdout.as_mut() {
                let _ = o.read_to_end(&mut buf);
            }
            buf
        });
        let err_t = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(e) = stderr.as_mut() {
                let _ = e.read_to_end(&mut buf);
            }
            buf
        });
        let deadline = Instant::now() + Duration::from_secs_f64(timeout);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout = out_t.join().unwrap_or_default();
                    let stderr = err_t.join().unwrap_or_default();
                    return Ok(CommandOutput {
                        status: status.code().unwrap_or(-1),
                        stdout,
                        stderr,
                    });
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        // Upstream raises RuntimeError(timeout) here — the
                        // partial output is discarded with the pipes.
                        let _ = out_t.join();
                        let _ = err_t.join();
                        return Err(SpawnError::TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(SpawnError::Other(e.to_string())),
            }
        }
    }
}

struct CommandOutput {
    status: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

use std::time::{Duration, Instant};
