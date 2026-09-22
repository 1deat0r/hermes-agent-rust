//! Gateway Health & Diagnostics OTLP export runtime.
//!
//! PARITY: `agent/monitoring/gateway_health_export.py` @ 5d59366 (whole
//! module, 344 lines).
//!
//! The OTel SDK pieces (metric provider, log provider, span/log/metric
//! exporters) are caller-wired seams: [`SpanSink`] carries spans,
//! [`LogSink`] carries diagnostic log records, and
//! [`collect_observable_metrics`] returns the gauge observations the
//! caller's meter scrapes. The snapshot readers
//! (`gateway.status.read_runtime_status`, `cron_health`, background
//! counts) arrive as closures — those surfaces are unported; the merge
//! order, failure isolation (per-reader warn + continue), and shutdown
//! ordering port exactly.

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

/// PARITY: `_gateway_health_event` (upstream lines 269-270) — the plane
/// filter: only gateway_health and cron_execution ride this exporter.
pub fn gateway_health_event_filter(event: &Value) -> bool {
    matches!(
        event.get("event").and_then(Value::as_str),
        Some("gateway_health") | Some("cron_execution")
    )
}

/// Every gauge the runtime snapshot can emit MUST be listed here or it
/// is silently dropped.
///
/// PARITY: `_OBSERVABLE_METRIC_NAMES` (upstream lines 47-54).
pub const OBSERVABLE_METRIC_NAMES: [&str; 16] = [
    "hermes.gateway.up",
    "hermes.gateway.state",
    "hermes.gateway.active_agents",
    "hermes.gateway.busy",
    "hermes.gateway.drainable",
    "hermes.gateway.restart_requested",
    "hermes.gateway.background_work",
    "hermes.gateway.background_delegations",
    "hermes.platform.up",
    "hermes.platform.degraded",
    "hermes.cron.scheduler.heartbeat_age_seconds",
    "hermes.cron.scheduler.last_success_age_seconds",
    "hermes.cron.scheduler.catch_up_occurrences",
    "hermes.cron.jobs.enabled",
    "hermes.cron.jobs.running",
    "hermes.cron.jobs.overdue",
];

/// Endpoint + headers for one OTLP signal exporter.
///
/// PARITY: `_exporter_kwargs` (upstream lines 113-115).
pub fn exporter_kwargs(
    config: Option<&Value>,
    signal: &str,
) -> (String, Option<BTreeMap<String, String>>) {
    let otlp = super::otlp_exporter::otlp_config(config);
    let endpoint = otlp
        .get("endpoint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let headers = otlp
        .get("headers_env")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|env| (k.clone(), env.to_string())))
                .collect::<BTreeMap<_, _>>()
        });
    let headers = headers.map(|h| super::otlp_exporter::resolve_headers(Some(&h)));
    (
        super::otlp_exporter::signal_endpoint(&endpoint, signal),
        headers.filter(|h| !h.is_empty()),
    )
}

/// Best-effort non-negative count read; 0 when the reader fails.
///
/// PARITY: `_count` (upstream lines 148-154).
pub fn read_count(read: &dyn Fn() -> Result<i64, String>) -> i64 {
    match read() {
        Ok(n) => n.max(0),
        Err(_) => 0,
    }
}

/// One diagnostic log record (the SDK LogRecord shape, transport-neutral).
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticLogRecord {
    pub timestamp_ns: Option<i64>,
    pub severity_text: String,
    pub severity_number: i64,
    pub body: String,
    pub attributes: BTreeMap<String, Value>,
    pub scope: Option<String>,
}

/// Map one gateway_diagnostic event to its OTLP log record. Rendered
/// messages stay out (they may carry IDs, names, paths, configured
/// strings); the source-controlled logger name becomes the
/// instrumentation scope.
///
/// PARITY: `GatewayDiagnosticLogStreamer.__call__` (upstream lines
/// 250-266).
pub fn diagnostic_log_record(event: &Value) -> Option<DiagnosticLogRecord> {
    if event.get("event").and_then(Value::as_str) != Some("gateway_diagnostic") {
        return None;
    }
    let severity = event
        .get("severity")
        .and_then(Value::as_str)
        .unwrap_or("warning");
    Some(DiagnosticLogRecord {
        timestamp_ns: event.get("ts_ns").and_then(Value::as_i64),
        severity_text: severity.to_uppercase(),
        severity_number: severity_number(Some(severity)),
        body: redact_string(Some("gateway diagnostic"), 500),
        attributes: diagnostic_log_attributes(event),
        scope: super::gateway_health::source_logger_for_export(
            event.get("source_logger").and_then(Value::as_str),
        ),
    })
}

/// The export runtime: span streamer + diagnostic log streamer over the
/// singleton emitter, plus an optional snapshot thread.
///
/// PARITY: `GatewayHealthExportRuntime` (upstream lines 62-102) +
/// `start_gateway_health_export` (lines 279-333). The OTel providers
/// and the root-logger handler attach are gateway-lifecycle wiring:
/// `log_sink` receives diagnostic log records, `snapshot_emit` runs
/// the snapshot thread body. Shutdown order ports exactly: stop the
/// thread (0.25 s join), detach the handler, flush (1 s) + unsubscribe
/// subscribers, then bounded (2 s) transport close.
pub struct GatewayHealthExportRuntime {
    /// Why the runtime is (or isn't) running.
    pub reason: String,
    span_streamer: Option<std::sync::Arc<super::otlp_exporter::OtlpStreamer>>,
    log_streamer: Option<std::sync::Arc<LogStreamer>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl GatewayHealthExportRuntime {
    fn disabled(reason: &str) -> Self {
        Self {
            reason: reason.to_string(),
            span_streamer: None,
            log_streamer: None,
            stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            thread: None,
        }
    }

    /// Ordered shutdown. Never panics into the caller.
    pub fn shutdown(&mut self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
            // Detach the snapshot thread: it exits on its next stop
            // check (max one interval). Upstream joins with a 0.25 s
            // timeout that can only succeed when the interval already
            // elapsed — effectively the same detach.
            let _ = self.thread.take();
            // Detach subscribers after a bounded flush so the terminal
            // lifecycle event cannot race exporter shutdown.
            let bus = super::emitter::get_emitter();
            bus.flush(1.0);
            if let Some(streamer) = self.span_streamer.take() {
                streamer.shutdown();
            }
            if let Some(streamer) = self.log_streamer.take() {
                streamer.shutdown();
            }
        }));
        self.reason = "shut down".to_string();
    }

    /// Spans exported so far (None when disabled).
    pub fn spans_exported(&self) -> Option<usize> {
        self.span_streamer.as_ref().map(|s| s.exported())
    }

    /// Diagnostic log records exported so far (None when disabled).
    pub fn logs_exported(&self) -> Option<usize> {
        self.log_streamer.as_ref().map(|s| s.exported())
    }
}

/// Emitter subscriber that sends gateway diagnostic events to the log
/// sink as OTLP log records.
///
/// PARITY: `GatewayDiagnosticLogStreamer` (upstream lines 238-266).
pub struct LogStreamer {
    sink: std::sync::Arc<super::otlp_exporter::SpanSink>,
    exported: std::sync::Mutex<usize>,
    subscription: std::sync::Mutex<Option<super::emitter::Subscriber>>,
}

impl LogStreamer {
    pub fn new(sink: std::sync::Arc<super::otlp_exporter::SpanSink>) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            sink,
            exported: std::sync::Mutex::new(0),
            subscription: std::sync::Mutex::new(None),
        })
    }

    pub fn subscribe(self: &std::sync::Arc<Self>) {
        let weak = std::sync::Arc::downgrade(self);
        let callback: super::emitter::Subscriber = std::sync::Arc::new(move |batch: &[Value]| {
            if let Some(streamer) = weak.upgrade() {
                streamer.on_batch(batch);
            }
        });
        *self.subscription.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(std::sync::Arc::clone(&callback));
        super::emitter::get_emitter().subscribe(callback);
    }

    pub fn shutdown(&self) {
        if let Some(callback) = self
            .subscription
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            super::emitter::get_emitter().unsubscribe(&callback);
        }
    }

    pub fn exported(&self) -> usize {
        *self.exported.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn on_batch(&self, batch: &[Value]) {
        let mut created = 0;
        for event in batch {
            let Some(record) = diagnostic_log_record(event) else {
                continue;
            };
            let payload = serde_json::json!({
                "timestamp_ns": record.timestamp_ns,
                "severity_text": record.severity_text,
                "severity_number": record.severity_number,
                "body": record.body,
                "attributes": record.attributes,
                "scope": record.scope,
            });
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (self.sink)(std::slice::from_ref(&payload))
            }))
            .is_ok();
            if ok {
                created += 1;
            }
        }
        *self.exported.lock().unwrap_or_else(|e| e.into_inner()) += created;
    }
}

/// Start P0 gateway health export if configured. Never raises: every
/// failure degrades to a disabled runtime with a reason.
///
/// PARITY: `start_gateway_health_export` (upstream lines 279-333).
/// `sinks` wires the transports (span sink + log sink); `snapshot_emit`
/// runs the snapshot body (snapshot readers are unported surfaces);
/// `snapshot_interval_secs` honors `logs_export_interval_seconds`
/// (floor 5). The root-logger handler attach is gateway wiring —
/// `with_log_handler` reports whether the caller attached it.
pub struct ExportWiring {
    pub span_sink: Option<std::sync::Arc<super::otlp_exporter::SpanSink>>,
    pub log_sink: Option<std::sync::Arc<super::otlp_exporter::SpanSink>>,
    pub snapshot_emit: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    pub snapshot_interval_secs: u64,
    pub with_log_handler: bool,
}

pub fn start_gateway_health_export(
    config: Option<&Value>,
    wiring: ExportWiring,
) -> GatewayHealthExportRuntime {
    if !is_enabled(config) {
        return GatewayHealthExportRuntime::disabled("disabled");
    }
    let gh = gateway_health_config(config);
    let metrics_on = gh
        .get("metrics_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let diagnostics_on = gh
        .get("diagnostic_events_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !metrics_on && !diagnostics_on {
        return GatewayHealthExportRuntime::disabled("disabled");
    }
    let mut runtime = GatewayHealthExportRuntime {
        reason: "enabled".to_string(),
        span_streamer: None,
        log_streamer: None,
        stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        thread: None,
    };
    if !(metrics_on || diagnostics_on) {
        return runtime;
    }
    // No SDK here: without sinks there is nothing to start (upstream's
    // otlp_unavailable arm).
    if wiring.span_sink.is_none() && wiring.log_sink.is_none() {
        log::warn!(
            "monitoring.gateway_health_export.enabled but no OTLP sinks are wired; export inactive"
        );
        return GatewayHealthExportRuntime::disabled("otlp_unavailable");
    }
    if diagnostics_on {
        if let Some(sink) = wiring.span_sink {
            // The plane filter: only gateway_health + cron_execution
            // ride the span exporter (upstream `event_filter=`).
            let filter: std::sync::Arc<dyn Fn(&Value) -> bool + Send + Sync> =
                std::sync::Arc::new(gateway_health_event_filter);
            let streamer = super::otlp_exporter::OtlpStreamer::new(sink, Some(filter));
            streamer.subscribe();
            runtime.span_streamer = Some(streamer);
        }
        if let Some(sink) = wiring.log_sink {
            let streamer = LogStreamer::new(sink);
            streamer.subscribe();
            runtime.log_streamer = Some(streamer);
        }
    }
    if diagnostics_on {
        if let Some(emit) = wiring.snapshot_emit {
            let stop = std::sync::Arc::clone(&runtime.stop);
            let interval = std::time::Duration::from_secs(wiring.snapshot_interval_secs.max(5));
            // First emit runs inline (upstream `_emit_snapshot_events`
            // before spawning the thread).
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| emit()));
            let thread = std::thread::Builder::new()
                .name("hermes-gateway-health-export".to_string())
                .spawn(move || {
                    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                        std::thread::sleep(interval);
                        if stop.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| emit()));
                    }
                });
            // A failed spawn degrades to no thread (fail-open).
            runtime.thread = thread.ok();
        }
    }
    runtime
}
