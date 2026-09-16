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
