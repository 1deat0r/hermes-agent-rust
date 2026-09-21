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
    // PARITY @ 5d59366 (live oracle): bare 13-char bearer passes the
    // 20-char floor (no vendor shape); the old un-floored fold is gone.
    assert_eq!(value, "Bearer abc123.def_ghi", "{value}");
    let event = json!({
        "event": "gateway_diagnostic",
        "error_code": "Bearer abcdefghijklmnopqrst1234",
    });
    let attrs = span_attrs(&event);
    let value = attrs["hermes.error_code"].as_str().unwrap();
    assert!(!value.contains("abcdefghijklmnopqrst1234"), "{value}");
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
fn a_panic_in_the_sink_is_fail_isolated_and_uncounted() {
    // PARITY @ 5d59366 (`export_batch` lines 198-208): `n += 1` sits
    // INSIDE the try — panicking maps contribute nothing but a debug
    // log, and the panic never propagates.
    let boom = |_batch: &[Value]| panic!("collector down");
    let created = export_batch(
        Some(&boom),
        &[
            json!({"event": "gateway_health"}),
            json!({"event": "gateway_health"}),
        ],
    );
    assert_eq!(created, 0);
    // Mixed batch: only the successful map counts.
    let calls = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let calls_cb = std::sync::Arc::clone(&calls);
    let flaky = move |_batch: &[Value]| {
        let mut n = calls_cb.lock().unwrap_or_else(|e| e.into_inner()); // poisoned by the first panic
        *n += 1;
        if *n == 1 {
            panic!("first span fails");
        }
    };
    let created = export_batch(
        Some(&flaky),
        &[
            json!({"event": "gateway_health"}),
            json!({"event": "gateway_health"}),
        ],
    );
    assert_eq!(created, 1);
}

// ── signal endpoint rewrite ────────────────────────────────────────────

#[test]
fn signal_endpoint_rewrites_traces_and_metrics_paths() {
    use hermes_agent::monitoring::otlp_exporter::signal_endpoint;
    assert_eq!(
        signal_endpoint("http://collector:4318/v1/traces", "logs"),
        "http://collector:4318/v1/logs"
    );
    assert_eq!(
        signal_endpoint("http://collector:4318/v1/metrics", "logs"),
        "http://collector:4318/v1/logs"
    );
    // Same-signal suffix is not "rewritten" (identity).
    assert_eq!(
        signal_endpoint("http://collector:4318/v1/traces", "traces"),
        "http://collector:4318/v1/traces"
    );
    // Other paths pass through untouched.
    assert_eq!(
        signal_endpoint("http://collector:4318/custom/path", "logs"),
        "http://collector:4318/custom/path"
    );
    assert_eq!(
        signal_endpoint("http://collector:4318", "logs"),
        "http://collector:4318"
    );
}

// ── streaming lifecycle ────────────────────────────────────────────────

#[test]
fn streamer_pushes_filtered_batches_and_detaches() {
    use hermes_agent::monitoring::emitter::get_emitter;
    use hermes_agent::monitoring::events::{GatewayDiagnosticEvent, GatewayHealthEvent};
    use hermes_agent::monitoring::otlp_exporter::start_streaming;
    use std::sync::{Arc, Mutex};
    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let seen_cb = Arc::clone(&seen);
    let sink: Arc<hermes_agent::monitoring::otlp_exporter::SpanSink> =
        Arc::new(move |batch: &[Value]| {
            seen_cb.lock().unwrap().extend(batch.iter().cloned());
        });
    let config =
        json!({"monitoring": {"export": {"otlp": {"enabled": true, "endpoint": "http://x:4318"}}}});
    let filter: Arc<dyn Fn(&Value) -> bool + Send + Sync> =
        Arc::new(|ev: &Value| ev.get("event").and_then(Value::as_str) == Some("gateway_health"));
    let streamer = start_streaming(Some(&config), Some(sink), Some(filter)).expect("streamer");
    // Through the live emitter: health passes the filter, diagnostic does not.
    get_emitter().emit(&GatewayHealthEvent::new());
    get_emitter().emit(&GatewayDiagnosticEvent::new("broker", "otlp"));
    get_emitter().flush(2.0);
    assert_eq!(streamer.exported(), 1);
    assert_eq!(seen.lock().unwrap().len(), 1);
    // Detach: further emits never reach the sink (idempotent shutdown).
    streamer.shutdown();
    streamer.shutdown();
    get_emitter().emit(&GatewayHealthEvent::new());
    get_emitter().flush(2.0);
    assert_eq!(streamer.exported(), 1);
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn start_streaming_noops_without_config_or_sink() {
    use hermes_agent::monitoring::otlp_exporter::start_streaming;
    let config =
        json!({"monitoring": {"export": {"otlp": {"enabled": true, "endpoint": "http://x:4318"}}}});
    assert!(start_streaming(None, None, None).is_none());
    assert!(
        start_streaming(Some(&config), None, None).is_none(),
        "no sink → warn + no-op"
    );
    let off = json!({"monitoring": {"export": {"otlp": {"enabled": false, "endpoint": "http://x:4318"}}}});
    assert!(start_streaming(Some(&off), None, None).is_none());
}
