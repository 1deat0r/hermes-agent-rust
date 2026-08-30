//! Parity tests for `agent/monitoring/otlp_exporter.py` (partial port) @
//! b9aa928. The SDK transport (provider/streamer/`start_streaming`) is
//! PENDING behind the caller-wired `SpanSink` seam; these cases cover the
//! ported pure logic. Env tests serialize behind a mutex per the workspace
//! convention.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use hermes_agent::monitoring::otlp_exporter::{
    export_batch, is_enabled, otlp_config, resolve_headers, span_attrs,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── config probe ─────────────────────────────────────────────────────────

#[test]
fn otlp_config_digs_three_levels_with_empty_fallbacks() {
    let config = json!({
        "monitoring": {"export": {"otlp": {"enabled": true, "endpoint": "http://x"}}}
    });
    let otlp = otlp_config(Some(&config));
    assert_eq!(otlp["endpoint"], "http://x");
    // Missing at any level -> empty object.
    assert!(otlp_config(Some(&json!({})))
        .as_object()
        .unwrap()
        .is_empty());
    assert!(otlp_config(Some(&json!({"monitoring": {}})))
        .as_object()
        .unwrap()
        .is_empty());
    assert!(otlp_config(None).as_object().unwrap().is_empty());
}

#[test]
fn enabled_requires_both_flag_and_endpoint() {
    assert!(!is_enabled(Some(&json!({
        "monitoring": {"export": {"otlp": {"enabled": true}}}
    }))));
    assert!(!is_enabled(Some(&json!({
        "monitoring": {"export": {"otlp": {"endpoint": "http://x"}}}
    }))));
    assert!(is_enabled(Some(&json!({
        "monitoring": {"export": {"otlp": {"enabled": true, "endpoint": "http://x"}}}
    }))));
}

// ── header resolution ────────────────────────────────────────────────────

#[test]
fn headers_resolve_from_env_and_skip_missing() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("TEST_OTLP_TOKEN_ENV", "sekrit-value");
        std::env::remove_var("TEST_OTLP_MISSING_ENV");
    }
    let mut headers_env = BTreeMap::new();
    headers_env.insert(
        "Authorization".to_string(),
        "TEST_OTLP_TOKEN_ENV".to_string(),
    );
    headers_env.insert("X-Org".to_string(), "TEST_OTLP_MISSING_ENV".to_string());

    let resolved = resolve_headers(Some(&headers_env));
    assert_eq!(
        resolved.get("Authorization").map(String::as_str),
        Some("sekrit-value")
    );
    // Missing env vars are skipped, not errors.
    assert!(!resolved.contains_key("X-Org"));
    unsafe {
        std::env::remove_var("TEST_OTLP_TOKEN_ENV");
    }
    // The config carried only env names — never values.
    assert!(!serde_json::to_string(&headers_env)
        .unwrap()
        .contains("sekrit"));
}

// ── span attribute mapping ───────────────────────────────────────────────

#[test]
fn span_attrs_keep_lists_by_kind() {
    let health = json!({
        "event": "gateway_health",
        "name": "gateway.health_snapshot",
        "gateway_state": "running",
        "active_agents": 2,
        "pid": 4242,
        // Not on the keep list: must not be exported.
        "profile": "prod",
        "install_id": "raw-install-id",
    });
    let attrs = span_attrs(&health);
    assert_eq!(attrs["hermes.event"], "gateway_health");
    assert_eq!(attrs["hermes.name"], "gateway.health_snapshot");
    assert_eq!(attrs["hermes.gateway_state"], "running");
    assert_eq!(attrs["hermes.active_agents"], 2);
    assert_eq!(attrs["hermes.pid"], 4242);
    assert!(
        !attrs.contains_key("hermes.profile"),
        "profile is not on the keep list"
    );
    assert!(
        !attrs.contains_key("hermes.install_id"),
        "install id is not on the keep list"
    );

    // Diagnostic kind uses its own list.
    let diag = json!({
        "event": "gateway_diagnostic",
        "subsystem": "platform.telegram",
        "severity": "warning",
        "ts_ns": 7,
    });
    let attrs = span_attrs(&diag);
    assert_eq!(attrs["hermes.subsystem"], "platform.telegram");
    assert_eq!(attrs["hermes.severity"], "warning");
    assert!(!attrs.contains_key("hermes.ts_ns"), "ts_ns is not exported");

    // Unknown kinds keep only the discriminator.
    let attrs = span_attrs(&json!({"event": "mystery", "secret": "x"}));
    assert_eq!(attrs["hermes.event"], "mystery");
    assert_eq!(attrs.len(), 1);
}

#[test]
fn span_string_values_are_redacted_and_bounded() {
    let event = json!({
        "event": "gateway_diagnostic",
        "error_code": "Bearer abc123.def_ghi",
    });
    let attrs = span_attrs(&event);
    let value = attrs["hermes.error_code"].as_str().unwrap();
    assert!(!value.contains("abc123"), "{value}");
    // 500-char bound.
    let event = json!({
        "event": "cron_execution",
        "status": "x".repeat(600),
    });
    let attrs = span_attrs(&event);
    assert_eq!(
        attrs["hermes.status"].as_str().unwrap().chars().count(),
        500
    );
}

// ── batch export over the sink seam ──────────────────────────────────────

#[test]
fn export_batch_maps_and_counts_over_the_sink() {
    let sink: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink_cb = Arc::clone(&sink);
    let sink_fn = move |batch: &[Value]| {
        sink_cb.lock().unwrap().extend(batch.iter().cloned());
    };
    let batch = vec![
        json!({"event": "gateway_health", "name": "snap"}),
        json!({"event": "cron_execution", "status": "ok"}),
    ];
    let created = export_batch(Some(&sink_fn), &batch);
    assert_eq!(created, 2);
    let received = sink.lock().unwrap();
    assert_eq!(received[0]["name"], "hermes.gateway_health");
    assert_eq!(received[1]["name"], "hermes.cron_execution");
    assert!(received[0]["attributes"]["hermes.name"] == "snap");
}

#[test]
fn export_batch_without_a_sink_creates_nothing() {
    // The fail-isolated arm: no wired transport, zero spans, no panic.
    assert_eq!(export_batch(None, &[json!({"event": "gateway_health"})]), 0);
}

#[test]
fn a_panic_in_the_sink_is_fail_isolated() {
    let boom = |_batch: &[Value]| panic!("collector down");
    // `export_batch` must not propagate the panic; count still reflects the
    // attempted maps (per-event try arm).
    let created = export_batch(
        Some(&boom),
        &[
            json!({"event": "gateway_health"}),
            json!({"event": "gateway_health"}),
        ],
    );
    assert_eq!(created, 2);
}
