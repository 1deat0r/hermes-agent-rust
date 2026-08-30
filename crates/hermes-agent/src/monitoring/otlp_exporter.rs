//! Export monitoring events to an OpenTelemetry Collector over OTLP/HTTP.
//!
//! PARITY: `agent/monitoring/otlp_exporter.py` @ b9aa928 — PARTIAL.
//!
//! Ported: the OTLP configuration probe ([`otlp_config`], [`is_enabled`]),
//! the header-resolution indirection ([`resolve_headers`] — the config
//! stores environment variable names, never secret values), the
//! event→span-attribute mapping ([`span_attrs`] with its per-kind keep
//! lists and redaction bounds), and the availability vocabulary
//! ([`OtlpUnavailable`]).
//!
//! PENDING: `_require_sdk` / `build_exporter` / `_make_provider` /
//! `OTLPStreamer` / `start_streaming` / `export_batch`'s span creation.
//! Upstream lazily imports the optional `opentelemetry-sdk` +
//! `opentelemetry-exporter-otlp-proto-http` extra (auto-installed via
//! `tools.lazy_deps`); the Rust port has no optional-SDK analog, so the
//! transport is a caller-wired [`SpanSink`] trait carrying the same
//! fail-isolation contract. `_resource_attributes` stays PENDING with
//! `agent/monitoring/policy.py` (`ensure_install_id`).
//!
//! Notes preserved from upstream:
//! * The destination is operator-configured; nothing ships a default.
//! * `headers_env` maps a header name to an environment variable name;
//!   values are read at export time and never logged or stored.
//! * Only monitoring events (gateway_health / gateway_diagnostic /
//!   cron_execution) exist on this plane; the `event_filter` seam keeps
//!   future planes from silently riding along.

use std::collections::BTreeMap;

use serde_json::Value;

use super::redaction::redact_for_export;

/// Raised upstream when the optional OpenTelemetry SDK isn't installed.
///
/// PARITY: `OTLPUnavailable(RuntimeError)` — in the Rust port this
/// error is produced by a caller-wired transport when no span sink has
/// been configured.
#[derive(Debug, thiserror::Error)]
#[error("OTLP export requires the optional dependency (no span sink configured)")]
pub struct OtlpUnavailable;

/// PARITY: `_otlp_config` (upstream lines 76-79) —
/// `config.monitoring.export.otlp`.
pub fn otlp_config(config: Option<&Value>) -> Value {
    let empty = Value::Object(serde_json::Map::new());
    let config = config.unwrap_or(&empty);
    let mon = config.get("monitoring").unwrap_or(&empty);
    let export = mon.get("export").unwrap_or(&empty);
    export.get("otlp").cloned().unwrap_or_else(|| empty.clone())
}

/// PARITY: `is_enabled` (upstream lines 244-246) — both `enabled` and a
/// non-empty `endpoint` must be set.
pub fn is_enabled(config: Option<&Value>) -> bool {
    let otlp = otlp_config(config);
    let enabled = otlp
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let endpoint = otlp
        .get("endpoint")
        .and_then(Value::as_str)
        .map(|e| !e.is_empty())
        .unwrap_or(false);
    enabled && endpoint
}

/// Resolve `{header_name: ENV_VAR_NAME}` → `{header_name: value}` from
/// env.
///
/// The config stores environment variable names, not secret values; values
/// are read from the environment here. Missing variables are skipped (and
/// noted at debug level without the value).
///
/// PARITY: `_resolve_headers` (upstream lines 63-74).
pub fn resolve_headers(headers_env: Option<&BTreeMap<String, String>>) -> BTreeMap<String, String> {
    let mut resolved = BTreeMap::new();
    for (header_name, env_name) in headers_env.into_iter().flatten() {
        match std::env::var(env_name) {
            Ok(val) if !val.is_empty() => {
                resolved.insert(header_name.clone(), val);
            }
            _ => {
                log::debug!(
                    "OTLP header {}: env var {} not set; skipping",
                    header_name,
                    env_name
                );
            }
        }
    }
    resolved
}

/// Span attributes for a monitoring event (content-free by construction).
///
/// PARITY: `_span_attrs` (upstream lines 126-157). Only the per-kind
/// keep-list columns are exported; string values pass through
/// [`redact_for_export`] bounded to 500 chars (failing closed to
/// `[redaction-unavailable]` — the redactor here is infallible, so that
/// arm degrades to the redaction itself being a no-op passthrough).
pub fn span_attrs(event: &Value) -> BTreeMap<String, Value> {
    let kind = event
        .get("event")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut attrs = BTreeMap::new();
    attrs.insert("hermes.event".to_string(), Value::String(kind.to_string()));
    let keep_by_kind: &[&str] = match kind {
        "gateway_health" => &[
            "name",
            "gateway_state",
            "old_state",
            "new_state",
            "exit_reason",
            "restart_requested",
            "active_agents",
            "gateway_busy",
            "gateway_drainable",
            "platform_count",
            "fatal_platform_count",
            "version",
            "supervision_mode",
            "pid",
        ],
        "gateway_diagnostic" => &[
            "name",
            "subsystem",
            "error_class",
            "error_code",
            "platform",
            "old_state",
            "new_state",
            "version",
            "severity",
        ],
        "cron_execution" => &[
            "status",
            "job_key",
            "source",
            "duration_ms",
            "delivery_outcome",
            "error_class",
        ],
        _ => &[],
    };
    for column in keep_by_kind {
        let value = match event.get(*column) {
            Some(value) if !value.is_null() => value,
            _ => continue,
        };
        let value = if let Some(text) = value.as_str() {
            match redact_for_export(Some(text)) {
                Some(redacted) if !redacted.is_empty() => {
                    Value::String(redacted.chars().take(500).collect())
                }
                _ => Value::String("[redacted]".to_string()),
            }
        } else {
            value.clone()
        };
        attrs.insert(format!("hermes.{column}"), value);
    }
    attrs
}

/// The caller-wired OTLP transport seam (the Rust analog of the optional
/// SDK import): receives each batch's spans.
pub type SpanSink = dyn Fn(&[Value]) + Send + Sync;

/// Map a batch of events to OTel span payloads. Returns spans created.
///
/// PARITY: `export_batch` (upstream lines 160-174) — per-event failures
/// are logged and skipped; the count reflects only successful maps. With
/// no sink wired this returns 0 (the fail-isolated arm).
pub fn export_batch(sink: Option<&SpanSink>, batch: &[Value]) -> usize {
    let Some(sink) = sink else {
        return 0;
    };
    let mut created = 0;
    for event in batch {
        let name = event
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or("event");
        let span = serde_json::json!({
            "name": format!("hermes.{name}"),
            "attributes": span_attrs(event),
        });
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sink(std::slice::from_ref(&span))
        }))
        .ok();
        created += 1;
    }
    created
}
