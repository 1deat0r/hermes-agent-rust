//! Export monitoring events to an OpenTelemetry Collector over OTLP/HTTP.
//!
//! PARITY: `agent/monitoring/otlp_exporter.py` @ 5d59366 (whole module,
//! 283 lines).
//!
//! Ported: the OTLP configuration probe ([`otlp_config`], [`is_enabled`]),
//! the header-resolution indirection ([`resolve_headers`] — the config
//! stores environment variable names, never secret values), the
//! event→span-attribute mapping ([`span_attrs`] with its per-kind keep
//! lists and redaction bounds), the signal-endpoint rewrite
//! ([`signal_endpoint`]), the batch mapper ([`export_batch`] —
//! per-event failures are skipped and uncounted), and the streaming
//! lifecycle ([`OtlpStreamer`], [`start_streaming`]) over the
//! caller-wired [`SpanSink`] seam. The safe resource attributes live in
//! [`super::gateway_health_export::safe_resource_attributes`] (one
//! implementation; upstream keeps a parallel copy here).
//!
//! The OTel SDK itself (`_require_sdk` / `build_exporter` /
//! `_make_provider` — the optional `hermes-agent[otlp]` extra,
//! auto-installed via `tools.lazy_deps`) has no Rust analog: the SDK
//! transport IS the sink the caller wires. `headers_env` values are
//! read at export time and never logged or stored.
//!
//! Notes preserved from upstream:
//! * The destination is operator-configured; nothing ships a default.
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

/// Map a batch of events to OTel span payloads. Returns spans created —
/// only successful maps count (a panicking sink contributes nothing but
/// a debug log, exactly like upstream's `try/except` around
/// `start_span().end()`).
///
/// PARITY: `export_batch` (upstream lines 198-208).
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
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sink(std::slice::from_ref(&span))
        }))
        .is_ok();
        if ok {
            created += 1;
        } else {
            log::debug!("OTLP span map failed");
        }
    }
    created
}

/// Rewrite a traces/metrics OTLP path to `/v1/<signal>`; other paths
/// pass through.
///
/// PARITY: `_signal_endpoint` (upstream lines 98-104).
pub fn signal_endpoint(endpoint: &str, signal: &str) -> String {
    const SUFFIXES: [&str; 2] = ["/v1/traces", "/v1/metrics"];
    let target = format!("/v1/{signal}");
    for suffix in SUFFIXES {
        if suffix != target && endpoint.ends_with(suffix) {
            return format!("{}{target}", &endpoint[..endpoint.len() - suffix.len()]);
        }
    }
    endpoint.to_string()
}

use std::sync::{Arc, Mutex};

use super::emitter::{get_emitter, Subscriber};

/// A live subscriber that pushes each emitter batch to the sink as
/// spans. Register with `emitter.subscribe(streamer.as_subscriber())`.
/// Fail-isolated by the emitter; `exported` counts spans created.
///
/// PARITY: `EmitterStreamer` + `OTLPStreamer` (upstream lines 212-241).
/// The OTel provider/processor pair IS the caller-wired sink here.
pub struct OtlpStreamer {
    sink: Arc<SpanSink>,
    filter: Option<Arc<dyn Fn(&Value) -> bool + Send + Sync>>,
    exported: Mutex<usize>,
    subscription: Mutex<Option<Subscriber>>,
}

impl OtlpStreamer {
    pub fn new(
        sink: Arc<SpanSink>,
        filter: Option<Arc<dyn Fn(&Value) -> bool + Send + Sync>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            sink,
            filter,
            exported: Mutex::new(0),
            subscription: Mutex::new(None),
        })
    }

    /// Attach to the singleton emitter (idempotent).
    pub fn subscribe(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        let callback: Subscriber = Arc::new(move |batch: &[Value]| {
            if let Some(streamer) = weak.upgrade() {
                streamer.on_batch(batch);
            }
        });
        *self.subscription.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&callback));
        get_emitter().subscribe(callback);
    }

    /// Detach from the singleton emitter (idempotent).
    pub fn shutdown(&self) {
        if let Some(callback) = self
            .subscription
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            get_emitter().unsubscribe(&callback);
        }
    }

    /// Spans created so far.
    pub fn exported(&self) -> usize {
        *self.exported.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn on_batch(&self, batch: &[Value]) {
        let batch: Vec<Value> = match &self.filter {
            Some(filter) => batch.iter().filter(|ev| filter(ev)).cloned().collect(),
            None => batch.to_vec(),
        };
        if batch.is_empty() {
            return;
        }
        let created = export_batch(Some(self.sink.as_ref()), &batch);
        *self.exported.lock().unwrap_or_else(|e| e.into_inner()) += created;
    }
}

/// If OTLP is enabled, attach a streamer to the singleton emitter.
///
/// `event_filter` scopes the exporter to its plane. A
/// configured-but-missing transport is a caller-wiring concern: with no
/// sink this returns `None` and logs (never raises into startup) —
/// the same no-op contract as upstream's missing-SDK arm.
///
/// PARITY: `start_streaming` (upstream lines 258-278).
pub fn start_streaming(
    config: Option<&Value>,
    sink: Option<Arc<SpanSink>>,
    filter: Option<Arc<dyn Fn(&Value) -> bool + Send + Sync>>,
) -> Option<Arc<OtlpStreamer>> {
    if !is_enabled(config) {
        return None;
    }
    let Some(sink) = sink else {
        log::warn!(
            "monitoring.export.otlp.enabled but no span sink is wired; OTLP export inactive"
        );
        return None;
    };
    let streamer = OtlpStreamer::new(sink, filter);
    streamer.subscribe();
    Some(streamer)
}
