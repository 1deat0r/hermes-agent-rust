//! Gateway Health & Diagnostics OTLP export runtime — pure projection
//! helpers.
//!
//! PARITY: `agent/monitoring/gateway_health_export.py` @ b9aa928 — PARTIAL.
//!
//! Ported: the resource/diagnostic attribute allowlists, the
//! safe-resource-value grammar (values changed by redaction are rejected),
//! the config probes (`gateway_health_export` / `otlp` / `_enabled`), the
//! OTLP endpoint derivations (traces→metrics/logs), the supervision-mode
//! env probe, the severity→SeverityNumber mapping, and the
//! gateway-health event filter.
//!
//! PENDING: `GatewayHealthExportRuntime` / `start_gateway_health_export`
//! and the streamer/provider classes — they orchestrate the optional
//! opentelemetry SDK (see the otlp_exporter module note) plus background
//! snapshot threads and root-logger handler attachment, which are
//! gateway-lifecycle wiring. `_install_id` stays PENDING with
//! `policy.ensure_install_id`.

use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use super::gateway_health::safe_instance_id;
use super::redaction::redact_for_export;

/// PARITY: `_DEFAULT_DIAGNOSTIC_SCOPE` (upstream line 15).
pub const DEFAULT_DIAGNOSTIC_SCOPE: &str = "hermes.gateway.diagnostics";

/// PARITY: `_RESOURCE_ATTRIBUTE_KEYS` (upstream lines 17-26).
pub const RESOURCE_ATTRIBUTE_KEYS: [&str; 9] = [
    "service.name",
    "service.namespace",
    "service.version",
    "service.instance.id",
    "deployment.environment.name",
    "cloud.provider",
    "cloud.platform",
    "cloud.region",
    "telemetry.scope",
];

/// PARITY: `_DIAGNOSTIC_ATTRIBUTE_KEYS` (upstream lines 27-37).
pub const DIAGNOSTIC_ATTRIBUTE_KEYS: [&str; 9] = [
    "name",
    "subsystem",
    "error_class",
    "error_code",
    "platform",
    "old_state",
    "new_state",
    "version",
    "severity",
];

/// PARITY: `_SAFE_RESOURCE_VALUE` — `^[A-Za-z0-9._:/-]{1,128}$`.
static SAFE_RESOURCE_VALUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9._:/-]{1,128}$").expect("resource value re"));

/// PARITY: `_redact_string` (upstream lines 40-46).
pub fn redact_string(raw: Option<&str>, limit: usize) -> String {
    let text = raw.unwrap_or("");
    match redact_for_export(Some(text)) {
        Some(redacted) if !redacted.is_empty() => redacted.chars().take(limit).collect(),
        _ => "[redacted]".to_string(),
    }
}

/// Allowlist bounded resource labels and reject values changed by
/// redaction.
///
/// PARITY: `_safe_resource_attributes` (upstream lines 49-73). The
/// `service.instance.id` arm routes through the sha256 instance-id helper
/// so the source ID never exports raw.
pub fn safe_resource_attributes(raw: Option<&Value>) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(object) = raw.and_then(Value::as_object) else {
        return attrs;
    };
    for (key, value) in object {
        if !RESOURCE_ATTRIBUTE_KEYS.contains(&key.as_str()) || value.is_null() {
            continue;
        }
        if key == "service.instance.id" {
            attrs.insert(key.clone(), safe_instance_id(Some(value)));
            continue;
        }
        let text = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if !SAFE_RESOURCE_VALUE.is_match(&text) {
            continue;
        }
        // A value that redaction would change is not a safe static label.
        if redact_string(Some(&text), 128) != text {
            continue;
        }
        attrs.insert(key.clone(), text);
    }
    attrs
}

/// PARITY: `_diagnostic_log_attributes` (upstream lines 92-100) — only
/// allowlisted keys export; string values redact, non-strings pass.
pub fn diagnostic_log_attributes(event: &Value) -> BTreeMap<String, Value> {
    let mut attrs = BTreeMap::new();
    for key in DIAGNOSTIC_ATTRIBUTE_KEYS {
        let value = match event.get(key) {
            Some(value) if !value.is_null() => value,
            _ => continue,
        };
        let value = if let Some(text) = value.as_str() {
            Value::String(redact_string(Some(text), 500))
        } else {
            value.clone()
        };
        attrs.insert(format!("hermes.{key}"), value);
    }
    attrs
}

/// PARITY: `_gateway_health_config` (upstream lines 167-170) —
/// `config.monitoring.gateway_health_export`.
pub fn gateway_health_config(config: Option<&Value>) -> Value {
    let empty = Value::Object(serde_json::Map::new());
    let config = config.unwrap_or(&empty);
    let mon = config.get("monitoring").unwrap_or(&empty);
    mon.get("gateway_health_export")
        .cloned()
        .unwrap_or_else(|| empty.clone())
}

/// PARITY: `_enabled` (upstream lines 178-182) — both planes must be
/// enabled and the OTLP endpoint set.
pub fn is_enabled(config: Option<&Value>) -> bool {
    let gh = gateway_health_config(config);
    let gh_enabled = gh.get("enabled").and_then(Value::as_bool).unwrap_or(false);
    let otlp = super::otlp_exporter::otlp_config(config);
    let otlp_enabled = otlp
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let endpoint = otlp
        .get("endpoint")
        .and_then(Value::as_str)
        .map(|e| !e.is_empty())
        .unwrap_or(false);
    gh_enabled && otlp_enabled && endpoint
}

/// PARITY: `_metric_endpoint` (upstream lines 231-235) — a traces endpoint
/// is retargeted to metrics; anything else passes through.
pub fn metric_endpoint(endpoint: &str) -> String {
    if let Some(base) = endpoint.strip_suffix("/v1/traces") {
        return format!("{base}/v1/metrics");
    }
    endpoint.to_string()
}

/// PARITY: `_logs_endpoint` (upstream lines 237-243) — traces or metrics
/// endpoints retarget to logs; anything else passes through.
pub fn logs_endpoint(endpoint: &str) -> String {
    if let Some(base) = endpoint.strip_suffix("/v1/traces") {
        return format!("{base}/v1/logs");
    }
    if let Some(base) = endpoint.strip_suffix("/v1/metrics") {
        return format!("{base}/v1/logs");
    }
    endpoint.to_string()
}

/// PARITY: `_supervision_mode` (upstream lines 269-279) — env-probed in
/// the same precedence: systemd (INVOCATION_ID), s6, container, launchd,
/// then manual.
pub fn supervision_mode() -> &'static str {
    if std::env::var_os("INVOCATION_ID").is_some() {
        return "systemd";
    }
    if std::env::var_os("S6_CMD_ARG0").is_some() || std::env::var_os("S6_VERSION").is_some() {
        return "s6";
    }
    let container =
        std::env::var_os("container").is_some() || std::path::Path::new("/.dockerenv").exists();
    if container {
        return "container";
    }
    if std::env::var_os("LAUNCHD_SOCKET").is_some() {
        return "launchd";
    }
    "manual"
}

/// PARITY: `_severity_number` (upstream lines 471-483) — the OTel
/// SeverityNumber enum values (WARN=13, ERROR=17, FATAL=24, INFO=9,
/// DEBUG=5); anything unrecognized defaults to WARN.
pub fn severity_number(severity: Option<&str>) -> i64 {
    let sev = severity.unwrap_or("warning").to_lowercase();
    match sev.as_str() {
        "critical" | "fatal" => 24,
        "error" => 17,
        "info" | "information" => 9,
        "debug" => 5,
        _ => 13,
    }
}

/// PARITY: `_gateway_health_event` (upstream lines 629-631) — the plane
/// filter: only gateway_health and cron_execution ride this exporter.
pub fn gateway_health_event_filter(event: &Value) -> bool {
    matches!(
        event.get("event").and_then(Value::as_str),
        Some("gateway_health") | Some("cron_execution")
    )
}
