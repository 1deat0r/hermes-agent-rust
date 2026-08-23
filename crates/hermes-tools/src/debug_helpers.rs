//! Shared debug session infrastructure for Hermes tools.
//!
//! PARITY: tools/debug_helpers.py @ b9aa928 (105 LOC, ported 1:1).
//!
//! Replaces the identical DEBUG_MODE / _log_debug_call / _save_debug_log /
//! get_debug_session_info boilerplate previously duplicated across web_tools,
//! vision_tools, and image_generation_tool.
//!
//! Usage in a tool module (Rust: wrap in a `Lazy<Mutex<_>>` if shared):
//!
//! ```ignore
//! use hermes_tools::debug_helpers::DebugSession;
//!
//! let mut debug = DebugSession::new("web_tools", "WEB_TOOLS_DEBUG");
//! debug.log_call("web_search", json!({"query": q, "results": results}));
//! debug.save();
//! let info = debug.get_session_info();
//! ```

use hermes_constants::get_hermes_home;
use log::{debug, error};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Per-tool debug session that records tool calls to a JSON log file.
///
/// Activated by a tool-specific environment variable (e.g. WEB_TOOLS_DEBUG=true).
/// When disabled, all methods are cheap no-ops.
pub struct DebugSession {
    tool_name: String,
    enabled: bool,
    session_id: String,
    log_dir: PathBuf,
    calls: Vec<Value>,
    start_time: String,
}

impl DebugSession {
    /// Build a session. Enabled iff `env_var` is set to `"true"`
    /// (case-insensitive), matching `os.getenv(env_var, "false").lower() == "true"`.
    pub fn new(tool_name: &str, env_var: &str) -> DebugSession {
        let enabled = std::env::var(env_var)
            .unwrap_or_else(|_| "false".to_string())
            .to_ascii_lowercase()
            == "true";
        let session = DebugSession {
            tool_name: tool_name.to_string(),
            enabled,
            session_id: if enabled {
                new_uuid4_string()
            } else {
                String::new()
            },
            log_dir: get_hermes_home().join("logs"),
            calls: Vec::new(),
            start_time: if enabled {
                now_isoformat()
            } else {
                String::new()
            },
        };
        if session.enabled {
            // PARITY DIVERGENCE (documented): upstream `log_dir.mkdir(...)`
            // raises OSError when the dir cannot be created; here we degrade
            // to best-effort so a missing logs dir stays non-fatal. `save()`
            // is already fail-open upstream, so the observable outcome when
            // the dir is unusable is the same: no debug log file.
            if let Err(e) = std::fs::create_dir_all(&session.log_dir) {
                debug!(
                    "{} debug mode enabled - logs dir unavailable ({}): {}",
                    tool_name,
                    session.log_dir.display(),
                    e
                );
            } else {
                debug!(
                    "{} debug mode enabled - Session ID: {}",
                    tool_name, session.session_id
                );
            }
        }
        session
    }

    /// Whether this session is recording (mirrors the `active` property).
    pub fn active(&self) -> bool {
        self.enabled
    }

    /// Whether this session is recording.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The generated session id (empty when disabled).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The directory debug logs are written to (`<hermes_home>/logs`).
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    /// Redirect the log directory (mirrors upstream directly assigning
    /// `ds.log_dir = ...`; used by tests and the gateway cache layouts).
    pub fn set_log_dir(&mut self, dir: &Path) {
        self.log_dir = dir.to_path_buf();
    }

    /// Append a tool-call entry to the in-memory log.
    ///
    /// `call_data` entries are spread into the record (mirrors upstream
    /// `{..., **call_data}` — later keys win, so `call_data` may override
    /// `timestamp`/`tool_name` exactly like the Python dict spread).
    pub fn log_call(&mut self, call_name: &str, call_data: Value) {
        if !self.enabled {
            return;
        }
        let mut entry = Map::new();
        entry.insert("timestamp".to_string(), Value::String(now_isoformat()));
        entry.insert("tool_name".to_string(), Value::String(call_name.to_string()));
        if let Value::Object(map) = call_data {
            for (key, value) in map {
                entry.insert(key, value);
            }
        }
        // PARITY DIVERGENCE (documented): upstream `**call_data` raises
        // TypeError for a non-dict payload. `log_call` is typed Dict[str, Any]
        // at every call site, so the only reachable payload is an object; we
        // stay fail-open and ignore stray non-object values instead of
        // panicking.
        self.calls.push(Value::Object(entry));
    }

    /// Flush the in-memory log to a JSON file in the logs directory.
    ///
    /// Failure to write is logged and swallowed (upstream try/except around
    /// `json.dump`).
    pub fn save(&self) {
        if !self.enabled {
            return;
        }
        let filename = format!("{}_debug_{}.json", self.tool_name, self.session_id);
        let filepath = self.log_dir.join(filename);
        let mut payload = Map::new();
        payload.insert("session_id".to_string(), Value::String(self.session_id.clone()));
        payload.insert("start_time".to_string(), Value::String(self.start_time.clone()));
        payload.insert("end_time".to_string(), Value::String(now_isoformat()));
        payload.insert("debug_enabled".to_string(), Value::Bool(true));
        payload.insert("total_calls".to_string(), Value::from(self.calls.len() as u64));
        payload.insert("tool_calls".to_string(), Value::Array(self.calls.clone()));
        match serde_json::to_string_pretty(&Value::Object(payload)) {
            Ok(text) => match std::fs::write(&filepath, text) {
                Ok(()) => debug!("{} debug log saved: {}", self.tool_name, filepath.display()),
                Err(e) => error!("Error saving {} debug log: {}", self.tool_name, e),
            },
            Err(e) => error!("Error saving {} debug log: {}", self.tool_name, e),
        }
    }

    /// Return a summary dict suitable for returning from
    /// `get_debug_session_info()`.
    pub fn get_session_info(&self) -> Value {
        if !self.enabled {
            return json!({
                "enabled": false,
                "session_id": null,
                "log_path": null,
                "total_calls": 0,
            });
        }
        json!({
            "enabled": true,
            "session_id": self.session_id,
            "log_path": format!(
                "{}",
                self.log_dir
                    .join(format!(
                        "{}_debug_{}.json",
                        self.tool_name, self.session_id
                    ))
                    .display()
            ),
            "total_calls": self.calls.len(),
        })
    }
}

/// RFC 4122 version-4 UUID string, `uuid.uuid4()` equivalent without pulling
/// the `uuid` crate into hermes-tools: 16 random bytes from the OS CSPRNG
/// with version/variant bits set. Upstream raises OSError on entropy failure;
/// the `expect`s mirror that.
fn new_uuid4_string() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    let mut rng = std::fs::File::open("/dev/urandom").expect("open /dev/urandom");
    rng.read_exact(&mut bytes)
        .expect("read /dev/urandom");
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 10xx
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Local time in `YYYY-MM-DDTHH:MM:SS[.fraction]` form, mirroring
/// `datetime.datetime.now().isoformat()`.
///
/// PARITY DIVERGENCE (documented): Python emits microsecond precision
/// (6 digits, or none when zero); chrono emits nanosecond precision trimmed
/// to 0/3/6/9 digits. Both are naive local wall-clock ISO-8601 strings.
fn now_isoformat() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.f").to_string()
}
