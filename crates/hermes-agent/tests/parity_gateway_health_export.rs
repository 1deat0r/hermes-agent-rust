//! Parity tests for `agent/monitoring/gateway_health_export.py` (partial
//! port) @ b9aa928. Upstream has no dedicated test file (missing-test gap,
//! noted in the ledger); cases derive from the upstream code as oracle.

use serde_json::{json, Value};

use hermes_agent::monitoring::gateway_health_export::{
    diagnostic_log_attributes, gateway_health_config, gateway_health_event_filter, is_enabled,
    logs_endpoint, metric_endpoint, redact_string, safe_resource_attributes, severity_number,
    supervision_mode, DEFAULT_DIAGNOSTIC_SCOPE,
};

// ── redaction + resource allowlist ───────────────────────────────────────

#[test]
fn redact_string_scrubs_and_bounds() {
    // PARITY @ 5d59366: 6-char bearer passes the 20-char floor; a 20+
    // opaque folds. (Old un-floored fold removed with the inline sweep.)
    let out = redact_string(Some("Bearer abc123"), 500);
    assert_eq!(out, "Bearer abc123", "{out}");
    let out = redact_string(Some("Bearer abcdefghijklmnopqrst1234"), 500);
    assert!(!out.contains("abcdefghijklmnopqrst1234"), "{out}");
    let long = "a".repeat(600);
    assert_eq!(redact_string(Some(&long), 128).chars().count(), 128);
    assert_eq!(redact_string(None, 500), "[redacted]");
    assert_eq!(redact_string(Some(""), 500), "[redacted]");
}

#[test]
fn safe_resource_attributes_allowlist_and_grammar() {
    let raw = json!({
        "service.name": "hermes-gateway",
        "service.instance.id": "raw-install-id",
        "deployment.environment.name": "prod",
        "telemetry.scope": "gateway_health",
        // Not on the allowlist: dropped.
        "custom.label": "x",
        // Invalid grammar (space): dropped.
        "service.version": "not a valid value",
        // Null: dropped.
        "cloud.provider": null,
    });
    let attrs = safe_resource_attributes(Some(&raw));
    assert_eq!(
        attrs.get("service.name").map(String::as_str),
        Some("hermes-gateway")
    );
    assert_eq!(
        attrs.get("deployment.environment.name").map(String::as_str),
        Some("prod")
    );
    assert!(!attrs.contains_key("custom.label"));
    assert!(
        !attrs.contains_key("service.version"),
        "space fails the grammar"
    );
    assert!(!attrs.contains_key("cloud.provider"));
    // The instance id is content-free, never the raw value.
    let id = attrs.get("service.instance.id").unwrap();
    assert!(id.starts_with("sha256:"));
    assert!(!id.contains("raw-install-id"));
    // Non-dict input -> empty.
    assert!(safe_resource_attributes(Some(&json!([1]))).is_empty());
    assert!(safe_resource_attributes(None).is_empty());
}

#[test]
fn redaction_changed_values_are_rejected() {
    // A value whose redaction differs from itself is not a safe static
    // label (e.g. an e-mail-shaped string survives the grammar but not
    // redaction).
    let raw = json!({"service.namespace": "user@example.com"});
    let attrs = safe_resource_attributes(Some(&raw));
    assert!(!attrs.contains_key("service.namespace"), "{attrs:?}");
}

// ── diagnostic log attributes ────────────────────────────────────────────

#[test]
fn diagnostic_attributes_allowlist_with_redaction() {
    let event = json!({
        "name": "platform.fatal",
        "subsystem": "platform.telegram",
        "severity": "error",
        "error_code": "Bearer abcdefghijklmnopqrst1234",
        "profile": "prod",           // not allowlisted
        "ts_ns": 9,                  // not allowlisted
    });
    let attrs = diagnostic_log_attributes(&event);
    assert_eq!(attrs["hermes.name"], "platform.fatal");
    assert_eq!(attrs["hermes.subsystem"], "platform.telegram");
    assert_eq!(attrs["hermes.severity"], "error");
    let code = attrs["hermes.error_code"].as_str().unwrap();
    assert!(!code.contains("abcdefghijklmnopqrst1234"), "{code}");
    assert!(!attrs.contains_key("hermes.profile"));
    assert!(!attrs.contains_key("hermes.ts_ns"));
}

// ── config probes ────────────────────────────────────────────────────────

#[test]
fn gateway_health_config_probe_and_enabled_gate() {
    let full = json!({
        "monitoring": {
            "gateway_health_export": {"enabled": true},
            "export": {"otlp": {"enabled": true, "endpoint": "http://collector:4318"}}
        }
    });
    let gh = gateway_health_config(Some(&full));
    assert_eq!(gh["enabled"], true);
    assert!(is_enabled(Some(&full)));

    // Each gate alone is insufficient.
    let no_gh = json!({"monitoring": {"export": {"otlp": {"enabled": true, "endpoint": "x"}}}});
    assert!(!is_enabled(Some(&no_gh)));
    let no_otlp = json!({"monitoring": {"gateway_health_export": {"enabled": true}}});
    assert!(!is_enabled(Some(&no_otlp)));
    // No endpoint.
    let no_endpoint = json!({
        "monitoring": {
            "gateway_health_export": {"enabled": true},
            "export": {"otlp": {"enabled": true}}
        }
    });
    assert!(!is_enabled(Some(&no_endpoint)));
}

// ── endpoint derivation ──────────────────────────────────────────────────

#[test]
fn metric_and_log_endpoints_derive_from_traces() {
    assert_eq!(
        metric_endpoint("http://collector:4318/v1/traces"),
        "http://collector:4318/v1/metrics"
    );
    assert_eq!(
        metric_endpoint("http://collector:4318"),
        "http://collector:4318"
    );
    assert_eq!(
        logs_endpoint("http://collector:4318/v1/traces"),
        "http://collector:4318/v1/logs"
    );
    assert_eq!(
        logs_endpoint("http://collector:4318/v1/metrics"),
        "http://collector:4318/v1/logs"
    );
    assert_eq!(
        logs_endpoint("http://collector:4318"),
        "http://collector:4318"
    );
}

// ── severity mapping ─────────────────────────────────────────────────────

#[test]
fn severity_number_mapping_matches_otel_enum_values() {
    assert_eq!(severity_number(Some("critical")), 24);
    assert_eq!(severity_number(Some("fatal")), 24);
    assert_eq!(severity_number(Some("error")), 17);
    assert_eq!(severity_number(Some("information")), 9);
    assert_eq!(severity_number(Some("info")), 9);
    assert_eq!(severity_number(Some("debug")), 5);
    // Default (None, empty, unknown) is WARN.
    assert_eq!(severity_number(None), 13);
    assert_eq!(severity_number(Some("")), 13);
    assert_eq!(severity_number(Some("WARNING")), 13);
}

// ── plane filter ─────────────────────────────────────────────────────────

#[test]
fn gateway_health_event_filter_scopes_the_plane() {
    assert!(gateway_health_event_filter(
        &json!({"event": "gateway_health"})
    ));
    assert!(gateway_health_event_filter(
        &json!({"event": "cron_execution"})
    ));
    // Diagnostic events ride the log streamer, not this filter.
    assert!(!gateway_health_event_filter(
        &json!({"event": "gateway_diagnostic"})
    ));
    assert!(!gateway_health_event_filter(&json!({})));
}

#[test]
fn diagnostic_scope_constant_matches_upstream() {
    assert_eq!(DEFAULT_DIAGNOSTIC_SCOPE, "hermes.gateway.diagnostics");
}

// ── 5d59366 runtime ────────────────────────────────────────────────────

#[test]
fn exporter_kwargs_derive_signal_endpoints() {
    use hermes_agent::monitoring::gateway_health_export::exporter_kwargs;
    let config = json!({"monitoring": {"export": {"otlp": {
        "endpoint": "http://collector:4318/v1/traces",
        "headers_env": {"authorization": "OTLP_AUTH"},
    }}}});
    let (endpoint, _) = exporter_kwargs(Some(&config), "metrics");
    assert_eq!(endpoint, "http://collector:4318/v1/metrics");
    let (endpoint, _) = exporter_kwargs(Some(&config), "logs");
    assert_eq!(endpoint, "http://collector:4318/v1/logs");
    let (endpoint, headers) = exporter_kwargs(None, "traces");
    assert_eq!(endpoint, "");
    assert!(headers.is_none());
}

#[test]
fn observable_metric_names_pin_the_gauge_set() {
    use hermes_agent::monitoring::gateway_health_export::OBSERVABLE_METRIC_NAMES;
    assert_eq!(OBSERVABLE_METRIC_NAMES.len(), 16);
    assert!(OBSERVABLE_METRIC_NAMES.contains(&"hermes.gateway.up"));
    assert!(OBSERVABLE_METRIC_NAMES.contains(&"hermes.cron.jobs.overdue"));
}

#[test]
fn diagnostic_log_record_maps_and_filters() {
    use hermes_agent::monitoring::gateway_health_export::diagnostic_log_record;
    // Non-diagnostic events map to nothing.
    assert!(diagnostic_log_record(&json!({"event": "gateway_health"})).is_none());
    let record = diagnostic_log_record(&json!({
        "event": "gateway_diagnostic",
        "subsystem": "platform.telegram",
        "error_class": "auth_failed",
        "error_code": "auth_failed",
        "severity": "error",
        "ts_ns": 123,
        "source_logger": "gateway.platforms.telegram",
    }))
    .expect("record");
    assert_eq!(record.severity_text, "ERROR");
    assert_eq!(record.severity_number, 17);
    assert_eq!(record.timestamp_ns, Some(123));
    assert_eq!(record.scope.as_deref(), Some("gateway.platforms.telegram"));
    assert_eq!(record.attributes["hermes.subsystem"], "platform.telegram");
    // Rendered messages stay out; default severity is warning.
    let record = diagnostic_log_record(&json!({"event": "gateway_diagnostic"})).expect("record");
    assert_eq!(record.severity_text, "WARNING");
    assert_eq!(record.severity_number, 13);
}

#[test]
fn read_count_is_non_negative_or_zero() {
    use hermes_agent::monitoring::gateway_health_export::read_count;
    assert_eq!(read_count(&|| Ok(5)), 5);
    assert_eq!(read_count(&|| Ok(-3)), 0);
    assert_eq!(read_count(&|| Err("gone".to_string())), 0);
}

#[test]
fn export_runtime_lifecycle() {
    use hermes_agent::monitoring::gateway_health_export::{
        start_gateway_health_export, ExportWiring,
    };
    use std::sync::{Arc, Mutex};
    // Disabled config → disabled runtime with reason.
    let off = json!({"monitoring": {"gateway_health_export": {"enabled": false}}});
    let mut rt = start_gateway_health_export(
        Some(&off),
        ExportWiring {
            span_sink: None,
            log_sink: None,
            snapshot_emit: None,
            snapshot_interval_secs: 5,
            with_log_handler: false,
        },
    );
    assert_eq!(rt.reason, "disabled");
    assert!(rt.spans_exported().is_none());
    rt.shutdown();
    // Enabled but sinkless → otlp_unavailable (never raises).
    let on = json!({"monitoring": {
        "gateway_health_export": {"enabled": true},
        "export": {"otlp": {"enabled": true, "endpoint": "http://x:4318"}},
    }});
    let mut rt = start_gateway_health_export(
        Some(&on),
        ExportWiring {
            span_sink: None,
            log_sink: None,
            snapshot_emit: None,
            snapshot_interval_secs: 5,
            with_log_handler: false,
        },
    );
    assert_eq!(rt.reason, "otlp_unavailable");
    rt.shutdown();
    // Enabled with sinks → live span streamer through the emitter.
    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let seen_cb = Arc::clone(&seen);
    let sink: Arc<hermes_agent::monitoring::otlp_exporter::SpanSink> =
        Arc::new(move |batch: &[Value]| {
            seen_cb.lock().unwrap().extend(batch.iter().cloned());
        });
    let log_seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let log_seen_cb = Arc::clone(&log_seen);
    let log_sink: Arc<hermes_agent::monitoring::otlp_exporter::SpanSink> =
        Arc::new(move |batch: &[Value]| {
            log_seen_cb.lock().unwrap().extend(batch.iter().cloned());
        });
    let mut rt = start_gateway_health_export(
        Some(&on),
        ExportWiring {
            span_sink: Some(sink),
            log_sink: Some(log_sink),
            snapshot_emit: None,
            snapshot_interval_secs: 5,
            with_log_handler: false,
        },
    );
    assert_eq!(rt.reason, "enabled");
    hermes_agent::monitoring::emitter::get_emitter()
        .emit(&hermes_agent::monitoring::events::GatewayHealthEvent::new());
    hermes_agent::monitoring::emitter::get_emitter()
        .emit(&hermes_agent::monitoring::events::GatewayDiagnosticEvent::new("broker", "otlp"));
    hermes_agent::monitoring::emitter::get_emitter().flush(2.0);
    assert_eq!(rt.spans_exported(), Some(1), "health rides the span plane");
    assert_eq!(
        rt.logs_exported(),
        Some(1),
        "diagnostic rides the log plane"
    );
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(log_seen.lock().unwrap().len(), 1);
    rt.shutdown();
    assert_eq!(rt.reason, "shut down");
}
