//! Typed gateway monitoring events.
//!
//! PARITY: `agent/monitoring/events.py` @ b9aa928 (whole module).
//!
//! Content-free service-health and redacted diagnostic events for the
//! gateway daemon. These are the only event shapes the monitoring plane
//! emits: no prompts, messages, tool args/results, session history, or
//! usage analytics.

use serde_json::{json, Map, Value};

fn now_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Keys every event dict in insertion order (Python dataclass field order
/// + the leading `event` discriminator from `to_dict`).
fn event_map(event_kind: &str, fields: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    map.insert("event".to_string(), json!(event_kind));
    for (key, value) in fields {
        if !value.is_null() {
            map.insert((*key).to_string(), value.clone());
        } else {
            // Python `asdict` keeps None fields; mirror that.
            map.insert((*key).to_string(), Value::Null);
        }
    }
    Value::Object(map)
}

macro_rules! monitoring_event {
    ($name:ident, $kind:literal, $($field:ident : $ty:ty = $default:expr),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty,)*
            /// PARITY: `ts_ns: int = field(default_factory=_now_ns)`.
            pub ts_ns: i64,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    $($field: $default,)*
                    ts_ns: now_ns(),
                }
            }

            /// PARITY: `to_dict` — `{"event": $kind, **asdict(self)}`.
            pub fn to_dict(&self) -> Value {
                event_map($kind, &[$((stringify!($field), json!(self.$field)),)*
                    ("ts_ns", json!(self.ts_ns))])
            }
        }
    };
}

monitoring_event!(
    GatewayHealthEvent,
    "gateway_health",
    name: String = String::new(),
    gateway_state: Option<String> = None,
    old_state: Option<String> = None,
    new_state: Option<String> = None,
    exit_reason: Option<String> = None,
    restart_requested: Option<bool> = None,
    active_agents: i64 = 0,
    gateway_busy: bool = false,
    gateway_drainable: bool = false,
    platform_count: i64 = 0,
    fatal_platform_count: i64 = 0,
    profile: Option<String> = None,
    install_id: Option<String> = None,
    version: Option<String> = None,
    supervision_mode: Option<String> = None,
    pid: Option<i64> = None,
);

/// Redacted gateway diagnostic event for operator-owned observability.
///
/// PARITY: `GatewayDiagnosticEvent` (upstream lines 40-56).
#[derive(Debug, Clone)]
pub struct GatewayDiagnosticEvent {
    pub name: String,
    pub subsystem: String,
    /// PARITY: `error_class: str = "unknown"` (upstream line 43).
    pub error_class: String,
    pub error_code: Option<String>,
    pub platform: Option<String>,
    pub old_state: Option<String>,
    pub new_state: Option<String>,
    pub profile: Option<String>,
    pub version: Option<String>,
    pub severity: String,
    /// PARITY: `ts_ns: int = field(default_factory=_now_ns)`.
    pub ts_ns: i64,
    pub source_logger: Option<String>,
}

impl GatewayDiagnosticEvent {
    pub fn new(name: impl Into<String>, subsystem: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subsystem: subsystem.into(),
            error_class: "unknown".to_string(),
            error_code: None,
            platform: None,
            old_state: None,
            new_state: None,
            profile: None,
            version: None,
            severity: "warning".to_string(),
            ts_ns: now_ns(),
            source_logger: None,
        }
    }

    /// PARITY: `to_dict` — `{"event": "gateway_diagnostic", **asdict(self)}`.
    pub fn to_dict(&self) -> Value {
        event_map(
            "gateway_diagnostic",
            &[
                ("name", json!(self.name)),
                ("subsystem", json!(self.subsystem)),
                ("error_class", json!(self.error_class)),
                ("error_code", json!(self.error_code)),
                ("platform", json!(self.platform)),
                ("old_state", json!(self.old_state)),
                ("new_state", json!(self.new_state)),
                ("profile", json!(self.profile)),
                ("version", json!(self.version)),
                ("severity", json!(self.severity)),
                ("ts_ns", json!(self.ts_ns)),
                ("source_logger", json!(self.source_logger)),
            ],
        )
    }
}

/// Content-free durable cron execution lifecycle projection.
///
/// PARITY: `CronExecutionEvent` (upstream lines 59-71).
#[derive(Debug, Clone)]
pub struct CronExecutionEvent {
    pub status: String,
    pub job_key: String,
    /// PARITY: `source: str = "unknown"` (upstream line 62).
    pub source: String,
    pub duration_ms: Option<i64>,
    pub delivery_outcome: Option<String>,
    pub error_class: Option<String>,
    /// PARITY: `ts_ns: int = field(default_factory=_now_ns)`.
    pub ts_ns: i64,
}

impl CronExecutionEvent {
    pub fn new(status: impl Into<String>, job_key: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            job_key: job_key.into(),
            source: "unknown".to_string(),
            duration_ms: None,
            delivery_outcome: None,
            error_class: None,
            ts_ns: now_ns(),
        }
    }

    /// PARITY: `to_dict` — `{"event": "cron_execution", **asdict(self)}`.
    pub fn to_dict(&self) -> Value {
        event_map(
            "cron_execution",
            &[
                ("status", json!(self.status)),
                ("job_key", json!(self.job_key)),
                ("source", json!(self.source)),
                ("duration_ms", json!(self.duration_ms)),
                ("delivery_outcome", json!(self.delivery_outcome)),
                ("error_class", json!(self.error_class)),
                ("ts_ns", json!(self.ts_ns)),
            ],
        )
    }
}
