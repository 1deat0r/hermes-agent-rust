//! Gateway health and diagnostics signal producer.
//!
//! PARITY: `agent/monitoring/gateway_health.py` @ b9aa928 (whole module).
//!
//! This module keeps the plane narrow: service health monitoring plus
//! redacted operational diagnostics. It reuses the existing gateway
//! runtime-status contract and emits content-free metrics/events. No
//! prompts, messages, tool args, session history, audit records, or
//! product analytics belong here.
//!
//! TRANSLATION NOTES:
//! - Upstream wraps `gateway.status` reads in try/except with inline
//!   fallbacks; the Rust port IS those fallbacks (the module has no
//!   `gateway.status` to import yet, so the fallback paths are the code
//!   paths).
//! - `_safe_profile` / `_safe_version` resolve via `hermes_cli.profiles`
//!   and `hermes_cli.__version__`; the version is a compile-time constant
//!   here and the profile falls back to `"default"` until the profiles
//!   surface ports.
//! - `GatewayDiagnosticLogHandler` (a `logging.Handler` subclass) becomes
//!   [`diagnostic_event_for_log`]: the log-facade wiring that calls it per
//!   record is a gateway-lifecycle concern.

use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::events::{GatewayDiagnosticEvent, GatewayHealthEvent};
use super::redaction::redact_for_export;

/// PARITY: `GatewayMetric` (upstream lines 22-26).
#[derive(Debug, Clone, PartialEq)]
pub struct GatewayMetric {
    pub name: String,
    /// Python `int | float` — one f64 carries both.
    pub value: f64,
    pub attributes: BTreeMap<String, String>,
}

/// An event from [`GatewayHealthSnapshot`]: the Python union
/// `GatewayHealthEvent | GatewayDiagnosticEvent`.
#[derive(Debug, Clone)]
pub enum GatewayHealthSnapshotEvent {
    Health(GatewayHealthEvent),
    Diagnostic(GatewayDiagnosticEvent),
}

impl GatewayHealthSnapshotEvent {
    pub fn to_dict(&self) -> Value {
        match self {
            Self::Health(event) => event.to_dict(),
            Self::Diagnostic(event) => event.to_dict(),
        }
    }
}

/// PARITY: `GatewayHealthSnapshot` (upstream lines 28-31).
#[derive(Debug, Clone, Default)]
pub struct GatewayHealthSnapshot {
    pub metrics: Vec<GatewayMetric>,
    pub events: Vec<GatewayHealthSnapshotEvent>,
}

const RUNNING_PLATFORM_STATES: [&str; 4] = ["running", "connected", "ok", "ready"];
const FATAL_PLATFORM_STATES: [&str; 4] = ["fatal", "degraded", "error", "failed"];
const KNOWN_GATEWAY_STATES: &[&str] = &[
    "starting",
    "draining",
    "stopping",
    "stopped",
    "startup_failed",
    "unknown",
    "running",
    "connected",
    "ok",
    "ready",
    "fatal",
    "degraded",
    "error",
    "failed",
];
const KNOWN_PLATFORM_STATES: &[&str] = &[
    "running",
    "connected",
    "ok",
    "ready",
    "fatal",
    "degraded",
    "error",
    "failed",
    "connecting",
    "disconnected",
    "disabled",
    "paused",
    "retrying",
    "unknown",
];
const SUPERVISION_MODES: [&str; 6] = ["systemd", "s6", "container", "launchd", "manual", "unknown"];

/// PARITY: `_SOURCE_LOGGER_RE` — `^gateway(?:\.[A-Za-z_][A-Za-z0-9_]*)*$`.
static SOURCE_LOGGER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^gateway(?:\.[A-Za-z_][A-Za-z0-9_]*)*$").expect("logger re"));

/// PARITY: `_allowed_logger` (upstream lines 40-42).
fn allowed_logger(name: &str) -> bool {
    name == "gateway" || name.starts_with("gateway.")
}

/// Return a bounded source-controlled gateway logger name for OTLP scope.
///
/// PARITY: `source_logger_for_export` (upstream lines 45-48).
pub fn source_logger_for_export(name: Option<&str>) -> Option<String> {
    let value = name.unwrap_or("");
    if value.len() <= 128 && SOURCE_LOGGER_RE.is_match(value) {
        Some(value.to_string())
    } else {
        None
    }
}

/// Redact gateway diagnostic free text for operator-owned export.
///
/// Single scrub path: everything goes through
/// [`redact_for_export`] (unconditional secrets + PII), then is
/// length-bounded.
///
/// PARITY: `redact_gateway_message` (upstream lines 51-62).
pub fn redact_gateway_message(message: Option<&str>) -> String {
    let out = redact_for_export(Some(message.unwrap_or("")));
    out.unwrap_or_else(|| "[redaction-unavailable]".to_string())
        .chars()
        .take(500)
        .collect()
}

/// PARITY: `classify_gateway_error` (upstream lines 65-90) — substring
/// cascade over the lowercased text.
pub fn classify_gateway_error(raw: Option<&Value>) -> String {
    let s = match raw {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(other) if !other.is_null() => other.to_string().to_lowercase(),
        _ => String::new(),
    };
    let contains = |needles: &[&str]| needles.iter().any(|n| s.contains(n));
    if contains(&["auth", "token", "unauthorized", "forbidden", "401", "403"]) {
        return "auth_failed".to_string();
    }
    if s.contains("rate") && s.contains("limit") {
        return "rate_limited".to_string();
    }
    if s.contains("timeout") || s.contains("timed out") {
        return "timeout".to_string();
    }
    if contains(&[
        "network",
        "connection",
        "dns",
        "socket",
        "connect call failed",
        "failed to connect",
        "cannot connect",
        "unreachable",
        "name resolution",
    ]) {
        return "network_error".to_string();
    }
    if contains(&["config", "missing", "invalid"]) {
        return "invalid_config".to_string();
    }
    if s.contains("startup") {
        return "startup_failed".to_string();
    }
    if s.contains("fatal") {
        return "platform_fatal".to_string();
    }
    "unknown".to_string()
}

/// Reduce free-form shutdown text to a bounded operational class.
///
/// PARITY: `classify_exit_reason` (upstream lines 93-112).
pub fn classify_exit_reason(
    raw: Option<&Value>,
    state: Option<&Value>,
    restart_requested: bool,
) -> Option<String> {
    if restart_requested {
        return Some("restart_requested".to_string());
    }
    let state_name = match state {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(other) if !other.is_null() => other.to_string().to_lowercase(),
        _ => String::new(),
    };
    if raw.is_none() && state_name != "startup_failed" {
        return None;
    }
    let classified = classify_gateway_error(raw);
    if state_name == "startup_failed" {
        return Some(if classified != "unknown" {
            classified
        } else {
            "startup_failed".to_string()
        });
    }
    let text = match raw {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(other) if !other.is_null() => other.to_string().to_lowercase(),
        _ => String::new(),
    };
    if text.contains("signal") || text.contains("sigterm") || text.contains("sigint") {
        return Some("signal".to_string());
    }
    if state_name == "stopped" && (text.contains("shutdown") || text.contains("stop")) {
        return Some("planned_stop".to_string());
    }
    Some(classified)
}

/// PARITY: `_bounded_state` (upstream lines 115-118) — `str(raw or
/// "unknown").lower()`, kept only when in the allowed vocabulary.
fn bounded_state(raw: Option<&Value>, allowed: &[&str]) -> String {
    let state = match raw {
        Some(Value::String(s)) if !s.is_empty() => s.to_lowercase(),
        Some(Value::String(_)) | None | Some(Value::Null) => "unknown".to_string(),
        Some(other) => other.to_string().to_lowercase(),
    };
    if allowed.contains(&state.as_str()) {
        state
    } else {
        "unknown".to_string()
    }
}

/// PARITY: `_safe_metric_value` (upstream lines 121-128) — redacted then
/// length-bounded, defaulting to "unknown".
fn safe_metric_value(raw: Option<&Value>, limit: usize) -> String {
    let text = match raw {
        Some(Value::String(s)) => s.clone(),
        Some(other) if !other.is_null() => other.to_string(),
        _ => String::new(),
    };
    let redacted = redact_for_export(Some(&text)).unwrap_or_else(|| "unknown".to_string());
    let redacted = if redacted.is_empty() {
        "unknown".to_string()
    } else {
        redacted
    };
    redacted.chars().take(limit).collect()
}

/// Return a stable opaque instance key without exporting the source ID.
///
/// PARITY: `_safe_instance_id` (upstream lines 131-135). Private upstream;
/// public here because `gateway_health_export._runtime_resource_attributes`
/// imports it across the module boundary.
pub fn safe_instance_id(raw: Option<&Value>) -> String {
    let text = match raw {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(other) if !other.is_null() => other.to_string(),
        _ => "unknown".to_string(),
    };
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{}", &hex[..24])
}

/// PARITY: `subsystem_for_logger` (upstream lines 146-158).
pub fn subsystem_for_logger(logger_name: &str) -> String {
    if logger_name == "gateway.relay" || logger_name.starts_with("gateway.relay.") {
        return "platform.relay".to_string();
    }
    if logger_name.starts_with("gateway.platforms.") {
        let parts: Vec<&str> = logger_name.split('.').collect();
        if parts.len() >= 3 && !parts[2].is_empty() {
            return format!("platform.{}", parts[2]);
        }
    }
    if logger_name.starts_with("gateway.platforms") {
        return "platform".to_string();
    }
    if logger_name.starts_with("gateway") {
        return "gateway".to_string();
    }
    "gateway".to_string()
}

/// PARITY: `platform_for_subsystem` (upstream lines 161-165).
pub fn platform_for_subsystem(subsystem: &str) -> Option<String> {
    if let Some(rest) = subsystem.strip_prefix("platform.") {
        let platform = rest.split_once('.').map_or(rest, |(head, _)| head);
        return if platform.is_empty() {
            None
        } else {
            Some(platform.to_string())
        };
    }
    None
}

/// PARITY: `_parse_active_agents` (upstream lines 168-178) — the fallback
/// arm of the try/except (`max(0, int(raw))`) is the code path here.
fn parse_active_agents(raw: Option<&Value>) -> i64 {
    match raw {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0).max(0),
        Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(0).max(0),
        Some(Value::Bool(b)) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// PARITY: `_derive_busy` fallback (upstream lines 186-188).
fn derive_busy(gateway_running: bool, gateway_state: &str, active_agents: i64) -> bool {
    gateway_running && gateway_state == "running" && active_agents > 0
}

/// PARITY: `_derive_drainable` fallback (upstream lines 198-199).
fn derive_drainable(gateway_running: bool, gateway_state: &str) -> bool {
    gateway_running && gateway_state == "running"
}

/// PARITY: `_base_attrs` (upstream lines 202-210). Upstream accepts
/// `profile` in its keyword signature but never places it in the attribute
/// dict; the parameter is likewise dropped here.
fn base_attrs(install_id: &str, version: &str, supervision_mode: &str) -> BTreeMap<String, String> {
    let mode = {
        let mode = if supervision_mode.is_empty() {
            "unknown"
        } else {
            supervision_mode
        }
        .to_lowercase();
        if SUPERVISION_MODES.contains(&mode.as_str()) {
            mode
        } else {
            "unknown".to_string()
        }
    };
    let mut attrs = BTreeMap::new();
    attrs.insert(
        "service.instance.id".to_string(),
        safe_instance_id(Some(&Value::String(install_id.to_string()))),
    );
    attrs.insert(
        "service.version".to_string(),
        safe_metric_value(Some(&Value::String(version.to_string())), 64),
    );
    attrs.insert("hermes.supervision_mode".to_string(), mode);
    attrs
}

/// PARITY: `_metric` (upstream lines 213-219) — `None` extras are skipped.
#[allow(clippy::too_many_arguments)]
fn metric(
    name: &str,
    value: f64,
    attrs: &BTreeMap<String, String>,
    extras: &[(&str, Option<&str>)],
) -> GatewayMetric {
    let mut out = attrs.clone();
    for (key, val) in extras {
        if let Some(val) = val {
            out.insert(
                (*key).to_string(),
                safe_metric_value(Some(&Value::String((*val).to_string())), 128),
            );
        }
    }
    GatewayMetric {
        name: name.to_string(),
        value,
        attributes: out,
    }
}

/// PARITY: `_coerce_pid` (upstream lines 333-340).
fn coerce_pid(raw: Option<&Value>) -> Option<i64> {
    let pid = match raw {
        Some(Value::Number(n)) => n.as_i64()?,
        Some(Value::String(s)) => s.parse::<i64>().ok()?,
        _ => return None,
    };
    if pid > 0 {
        Some(pid)
    } else {
        None
    }
}

/// Convert gateway_state.json-compatible runtime state into P0 signals.
///
/// PARITY: `build_gateway_health_snapshot` (upstream lines 222-298). The
/// `runtime or {}` guard maps a `None`/non-object runtime to empty.
pub fn build_gateway_health_snapshot(
    runtime: Option<&Value>,
    gateway_running: bool,
    profile: &str,
    install_id: &str,
    version: &str,
    supervision_mode: &str,
) -> GatewayHealthSnapshot {
    let empty = Value::Object(serde_json::Map::new());
    let runtime = match runtime {
        Some(rt) if rt.is_object() => rt,
        _ => &empty,
    };
    let gateway_state = bounded_state(runtime.get("gateway_state"), KNOWN_GATEWAY_STATES);
    let active_agents = parse_active_agents(runtime.get("active_agents"));
    let busy = derive_busy(gateway_running, &gateway_state, active_agents);
    let drainable = derive_drainable(gateway_running, &gateway_state);
    let empty_map = serde_json::Map::new();
    let platforms = runtime
        .get("platforms")
        .and_then(Value::as_object)
        .unwrap_or(&empty_map);
    let base = base_attrs(install_id, version, supervision_mode);

    let mut metrics = vec![
        metric(
            "hermes.gateway.up",
            if gateway_running { 1.0 } else { 0.0 },
            &base,
            &[],
        ),
        metric(
            "hermes.gateway.active_agents",
            active_agents as f64,
            &base,
            &[],
        ),
        metric(
            "hermes.gateway.busy",
            if busy { 1.0 } else { 0.0 },
            &base,
            &[],
        ),
        metric(
            "hermes.gateway.drainable",
            if drainable { 1.0 } else { 0.0 },
            &base,
            &[],
        ),
        metric(
            "hermes.gateway.restart_requested",
            if runtime
                .get("restart_requested")
                .map(|v| v.as_bool().unwrap_or(false))
                .unwrap_or(false)
            {
                1.0
            } else {
                0.0
            },
            &base,
            &[],
        ),
    ];
    // `if gateway_state:` — the bounded state is never empty, so this arm
    // always fires (state defaults to "unknown").
    metrics.push(metric(
        "hermes.gateway.state",
        1.0,
        &base,
        &[("hermes.gateway.state", Some(&gateway_state))],
    ));

    let mut fatal_count = 0;
    let mut events: Vec<GatewayHealthSnapshotEvent> = Vec::new();
    for (platform, pdata) in platforms {
        let empty_pdata = Value::Object(serde_json::Map::new());
        let pdata = if pdata.is_object() {
            pdata
        } else {
            &empty_pdata
        };
        let state = bounded_state(pdata.get("state"), KNOWN_PLATFORM_STATES);
        let raw_error = pdata
            .get("error_code")
            .filter(|v| !v.is_null())
            .or_else(|| pdata.get("error_message").filter(|v| !v.is_null()));
        let error_code = classify_gateway_error(raw_error);
        let is_up = RUNNING_PLATFORM_STATES.contains(&state.as_str());
        let is_degraded = FATAL_PLATFORM_STATES.contains(&state.as_str());
        if is_degraded {
            fatal_count += 1;
        }
        metrics.push(metric(
            "hermes.platform.up",
            if is_up { 1.0 } else { 0.0 },
            &base,
            &[
                ("hermes.platform", Some(platform)),
                ("hermes.platform.state", Some(&state)),
            ],
        ));
        metrics.push(metric(
            "hermes.platform.degraded",
            if is_degraded { 1.0 } else { 0.0 },
            &base,
            &[
                ("hermes.platform", Some(platform)),
                ("hermes.platform.state", Some(&state)),
                ("hermes.error_code", Some(&error_code)),
            ],
        ));
        if is_degraded {
            let mut diag =
                GatewayDiagnosticEvent::new("platform.fatal", format!("platform.{platform}"));
            diag.platform = Some(platform.clone());
            diag.error_code = Some(error_code.clone());
            // `classify_gateway_error(error_code or pdata.get("error_message"))`
            // — the classified code string is always truthy, so the message
            // arm is unreachable upstream too.
            let code_value = Value::String(error_code.clone());
            diag.error_class = classify_gateway_error(Some(&code_value));
            diag.profile = Some(profile.to_string());
            diag.version = Some(version.to_string());
            diag.severity = if state == "fatal" {
                "error".to_string()
            } else {
                "warning".to_string()
            };
            events.push(GatewayHealthSnapshotEvent::Diagnostic(diag));
        }
    }

    let mut health = GatewayHealthEvent::new();
    health.name = "gateway.health_snapshot".to_string();
    health.gateway_state = Some(gateway_state);
    health.active_agents = active_agents;
    health.gateway_busy = busy;
    health.gateway_drainable = drainable;
    health.platform_count = platforms.len() as i64;
    health.fatal_platform_count = fatal_count;
    health.profile = Some(profile.to_string());
    health.install_id = Some(install_id.to_string());
    health.version = Some(version.to_string());
    health.supervision_mode = Some(supervision_mode.to_string());
    health.pid = coerce_pid(runtime.get("pid"));
    // `events.insert(0, ...)` — the health snapshot leads.
    events.insert(0, GatewayHealthSnapshotEvent::Health(health));
    GatewayHealthSnapshot { metrics, events }
}

/// Emit immediate content-free gateway events for runtime status changes.
///
/// Called by gateway.status.write_runtime_status after persisting the new
/// status. Fully fail-open: failures never affect gateway status writes.
///
/// PARITY: `emit_runtime_status_transition` (upstream lines 315-331). The
/// profile/version resolution seams (`hermes_cli.profiles` /
/// `hermes_cli.__version__`) arrive as parameters here.
pub fn emit_runtime_status_transition(
    previous: Option<&Value>,
    current: &Value,
    profile: &str,
    version: &str,
) {
    let empty = Value::Object(serde_json::Map::new());
    let previous = previous.unwrap_or(&empty);
    let (prev_obj, cur_obj) = match (previous.as_object(), current.as_object()) {
        (Some(p), Some(c)) => (p, c),
        _ => return,
    };
    let emitter = super::emitter::get_emitter();
    let mut out: Vec<GatewayHealthSnapshotEvent> = Vec::new();

    let prev_state_raw = prev_obj.get("gateway_state");
    let new_state_raw = cur_obj.get("gateway_state");
    let old_gateway_state = prev_state_raw
        .filter(|v| !v.is_null())
        .map(|v| bounded_state(Some(v), KNOWN_GATEWAY_STATES));
    let new_gateway_state = new_state_raw
        .filter(|v| !v.is_null())
        .map(|v| bounded_state(Some(v), KNOWN_GATEWAY_STATES));

    let restart_requested = cur_obj
        .get("restart_requested")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if old_gateway_state != new_gateway_state {
        if let Some(new_state) = &new_gateway_state {
            let mut lifecycle = GatewayHealthEvent::new();
            lifecycle.name = "gateway.lifecycle".to_string();
            lifecycle.gateway_state = Some(new_state.clone());
            lifecycle.old_state = old_gateway_state.clone();
            lifecycle.new_state = Some(new_state.clone());
            lifecycle.exit_reason = classify_exit_reason(
                cur_obj.get("exit_reason"),
                Some(&Value::String(new_state.clone())),
                restart_requested,
            );
            lifecycle.restart_requested = Some(restart_requested);
            lifecycle.active_agents = parse_active_agents(cur_obj.get("active_agents"));
            lifecycle.profile = Some(profile.to_string());
            lifecycle.version = Some(version.to_string());
            lifecycle.pid = coerce_pid(cur_obj.get("pid"));
            out.push(GatewayHealthSnapshotEvent::Health(lifecycle));

            if new_state == "startup_failed" {
                let exit_reason_value = cur_obj
                    .get("exit_reason")
                    .filter(|v| !v.is_null())
                    .cloned()
                    .unwrap_or_else(|| Value::String("startup_failed".to_string()));
                let class = classify_gateway_error(Some(&exit_reason_value));
                let mut diag = GatewayDiagnosticEvent::new("gateway.startup_failed", "gateway");
                diag.error_class = class.clone();
                diag.error_code = Some(class);
                diag.profile = Some(profile.to_string());
                diag.version = Some(version.to_string());
                diag.severity = "error".to_string();
                out.push(GatewayHealthSnapshotEvent::Diagnostic(diag));
            }
            if new_state == "stopped" {
                let mut exit = GatewayHealthEvent::new();
                exit.name = "gateway.exit".to_string();
                exit.gateway_state = Some(new_state.clone());
                exit.old_state = old_gateway_state.clone();
                exit.new_state = Some(new_state.clone());
                exit.exit_reason = classify_exit_reason(
                    cur_obj.get("exit_reason"),
                    Some(&Value::String(new_state.clone())),
                    restart_requested,
                );
                exit.restart_requested = Some(restart_requested);
                exit.active_agents = parse_active_agents(cur_obj.get("active_agents"));
                exit.profile = Some(profile.to_string());
                exit.version = Some(version.to_string());
                exit.pid = coerce_pid(cur_obj.get("pid"));
                out.push(GatewayHealthSnapshotEvent::Health(exit));
            }
        }
    }

    let empty_map = serde_json::Map::new();
    let old_platforms = previous
        .get("platforms")
        .and_then(Value::as_object)
        .unwrap_or(&empty_map);
    let new_platforms = current
        .get("platforms")
        .and_then(Value::as_object)
        .unwrap_or(&empty_map);
    for (platform, pdata) in new_platforms {
        let empty_pdata = Value::Object(serde_json::Map::new());
        let pdata = if pdata.is_object() {
            pdata
        } else {
            &empty_pdata
        };
        let prev_raw = old_platforms.get(platform).unwrap_or(&empty_pdata);
        let prev = if prev_raw.is_object() {
            prev_raw
        } else {
            &empty_pdata
        };
        let old_state = prev
            .get("state")
            .filter(|v| !v.is_null())
            .map(|v| bounded_state(Some(v), KNOWN_PLATFORM_STATES));
        let new_state = pdata
            .get("state")
            .filter(|v| !v.is_null())
            .map(|v| bounded_state(Some(v), KNOWN_PLATFORM_STATES));
        if old_state == new_state || new_state.is_none() {
            continue;
        }
        let new_state = new_state.unwrap();
        let error_code = classify_gateway_error(
            pdata
                .get("error_code")
                .filter(|v| !v.is_null())
                .or_else(|| pdata.get("error_message").filter(|v| !v.is_null())),
        );
        let severity = if ["fatal", "failed", "error"].contains(&new_state.to_lowercase().as_str())
        {
            "error"
        } else {
            "warning"
        };
        let mut change =
            GatewayDiagnosticEvent::new("platform.state_change", format!("platform.{platform}"));
        change.platform = Some(platform.clone());
        change.old_state = old_state;
        change.new_state = Some(new_state.clone());
        change.error_code = Some(error_code.clone());
        change.error_class = error_code.clone();
        change.profile = Some(profile.to_string());
        change.version = Some(version.to_string());
        change.severity = severity.to_string();
        out.push(GatewayHealthSnapshotEvent::Diagnostic(change));

        if FATAL_PLATFORM_STATES.contains(&new_state.to_lowercase().as_str()) {
            let mut fatal =
                GatewayDiagnosticEvent::new("platform.fatal", format!("platform.{platform}"));
            fatal.platform = Some(platform.clone());
            fatal.error_code = Some(error_code.clone());
            fatal.error_class = error_code;
            fatal.profile = Some(profile.to_string());
            fatal.version = Some(version.to_string());
            fatal.severity = severity.to_string();
            out.push(GatewayHealthSnapshotEvent::Diagnostic(fatal));
        }
    }
    for event in out {
        match &event {
            GatewayHealthSnapshotEvent::Health(health) => emitter.emit(health),
            GatewayHealthSnapshotEvent::Diagnostic(diag) => emitter.emit(diag),
        }
    }
}

/// Build the diagnostic event a [`GatewayDiagnosticLogHandler`] would emit
/// for one log record.
///
/// PARITY: `GatewayDiagnosticLogHandler.emit` (upstream lines 343-375):
/// warning/error levels only, gateway-owned logger names only, the
/// subsystem/platform derived from the logger, error class from the
/// message, and the level name as severity. The log-facade subscriber that
/// feeds records here is gateway-lifecycle wiring.
pub fn diagnostic_event_for_log(
    logger_name: &str,
    level: &str,
    message: &str,
    profile: &str,
    version: &str,
) -> Option<GatewayDiagnosticEvent> {
    // `record.levelno < logging.WARNING` → drop; `INFO`/`DEBUG`/`NOTSET`.
    if !matches!(
        level.to_lowercase().as_str(),
        "warning" | "error" | "critical"
    ) {
        return None;
    }
    if !allowed_logger(logger_name) {
        return None;
    }
    let subsystem = subsystem_for_logger(logger_name);
    let error_class = classify_gateway_error(Some(&Value::String(message.to_string())));
    let mut event = GatewayDiagnosticEvent::new(
        format!("gateway.log.{}", level.to_lowercase()),
        subsystem.clone(),
    );
    event.source_logger = source_logger_for_export(Some(logger_name));
    event.platform = platform_for_subsystem(&subsystem);
    event.error_class = error_class.clone();
    event.error_code = Some(error_class);
    event.profile = Some(profile.to_string());
    event.version = Some(version.to_string());
    event.severity = level.to_lowercase();
    Some(event)
}
