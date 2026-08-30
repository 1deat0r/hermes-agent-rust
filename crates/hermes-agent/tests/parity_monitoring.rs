//! Parity tests for `agent/monitoring/{events,emitter,__init__}.py`
//! @ b9aa928.
//!
//! Upstream has no dedicated test files for these leaves (missing-test gap,
//! noted in the ledger); cases derive from the upstream code as oracle.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use hermes_agent::monitoring::emitter::{
    get_emitter, reset_emitter_for_tests, MonitoringEmitter, TelemetryEmitter, ToMonitoringDict,
};
use hermes_agent::monitoring::events::{
    CronExecutionEvent, GatewayDiagnosticEvent, GatewayHealthEvent,
};

// ── events ───────────────────────────────────────────────────────────────

#[test]
fn health_event_to_dict_pins_the_shape() {
    let mut event = GatewayHealthEvent::new();
    event.name = "gateway".to_string();
    event.gateway_state = Some("draining".to_string());
    event.old_state = Some("running".to_string());
    event.new_state = Some("draining".to_string());
    event.active_agents = 2;
    event.platform_count = 3;
    let dict = event.to_dict();
    let map = dict.as_object().unwrap();
    // `{"event": "gateway_health", **asdict(self)}` — every dataclass field
    // is present, discriminator first.
    assert_eq!(map["event"], "gateway_health");
    for key in [
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
        "profile",
        "install_id",
        "version",
        "supervision_mode",
        "pid",
        "ts_ns",
    ] {
        assert!(map.contains_key(key), "missing {key}: {dict}");
    }
    assert_eq!(map["active_agents"], 2);
    assert_eq!(map["platform_count"], 3);
}

#[test]
fn diagnostic_and_cron_events_carry_defaults() {
    let diag = GatewayDiagnosticEvent::new("broker", "otlp");
    let dict = diag.to_dict();
    assert_eq!(dict["event"], "gateway_diagnostic");
    assert_eq!(dict["error_class"], "unknown", "Python default");
    assert_eq!(dict["severity"], "warning", "Python default");
    assert!(dict["error_code"].is_null());

    let cron = CronExecutionEvent::new("ok", "job-1");
    let dict = cron.to_dict();
    assert_eq!(dict["event"], "cron_execution");
    assert_eq!(dict["source"], "unknown", "Python default");
    assert_eq!(dict["job_key"], "job-1");
    assert!(dict["duration_ms"].is_null());
}

#[test]
fn events_timestamp_at_construction() {
    let event = GatewayHealthEvent::new();
    assert!(event.ts_ns > 0);
    let later = GatewayHealthEvent::new();
    assert!(later.ts_ns >= event.ts_ns);
}

// ── emitter ──────────────────────────────────────────────────────────────

fn collect() -> (
    Arc<StdMutex<Vec<serde_json::Value>>>,
    hermes_agent::monitoring::emitter::Subscriber,
) {
    let sink: Arc<StdMutex<Vec<serde_json::Value>>> = Arc::default();
    let sink_for_cb = Arc::clone(&sink);
    let cb: hermes_agent::monitoring::emitter::Subscriber =
        Arc::new(move |batch: &[serde_json::Value]| {
            sink_for_cb.lock().unwrap().extend(batch.iter().cloned());
        });
    (sink, cb)
}

#[test]
fn disabled_emitter_drops_events_silently() {
    // The singleton starts disabled: producers are no-ops until a
    // subscriber attaches.
    let emitter = MonitoringEmitter::new(false);
    emitter.emit(&GatewayHealthEvent::new());
    emitter.flush(0.5);
    let stats = emitter.stats();
    assert_eq!(stats["queued"], 0);
    assert_eq!(stats["dispatched"], 0);
}

#[test]
fn events_flow_to_subscribers_and_stats_count() {
    let emitter = MonitoringEmitter::new(true);
    let (sink, cb) = collect();
    emitter.subscribe(cb);
    for _ in 0..5 {
        let mut event = GatewayHealthEvent::new();
        event.name = "gw".to_string();
        emitter.emit(&event);
    }
    emitter.flush(2.0);
    assert_eq!(sink.lock().unwrap().len(), 5);
    let stats = emitter.stats();
    assert_eq!(stats["dispatched"], 5);
    assert_eq!(stats["dropped"], 0);
    assert_eq!(stats["subscribers"], 1);
    assert!(sink.lock().unwrap()[0]["event"] == "gateway_health");
    // setdefault("ts_ns", ...) — every payload carries a timestamp.
    assert!(sink.lock().unwrap()[0]["ts_ns"].as_i64().unwrap() > 0);
    emitter.close();
}

#[test]
fn raising_subscriber_is_fail_isolated() {
    // A slow or raising subscriber never affects the hot path or its
    // peers.
    let emitter = MonitoringEmitter::new(true);
    let boom: hermes_agent::monitoring::emitter::Subscriber =
        Arc::new(|_: &[serde_json::Value]| panic!("subscriber exploded"));
    let (sink, good) = collect();
    emitter.subscribe(boom);
    emitter.subscribe(good);
    emitter.emit(&GatewayDiagnosticEvent::new("a", "b"));
    emitter.flush(2.0);
    assert_eq!(sink.lock().unwrap().len(), 1, "peers unaffected");
    emitter.close();
}

#[test]
fn unsubscribing_the_last_subscriber_disables_collection() {
    let emitter = MonitoringEmitter::new(true);
    let (sink, cb) = collect();
    emitter.subscribe(Arc::clone(&cb));
    emitter.unsubscribe(&cb);
    assert_eq!(emitter.stats()["subscribers"], 0);
    emitter.emit(&CronExecutionEvent::new("ok", "j"));
    emitter.flush(0.5);
    assert!(sink.lock().unwrap().is_empty(), "collection disabled");
}

#[test]
fn plain_value_emit_carries_ts_ns_via_setdefault() {
    let emitter = MonitoringEmitter::new(true);
    let (sink, cb) = collect();
    emitter.subscribe(cb);
    // Plain dict without ts_ns — setdefault fills it.
    emitter.emit_value(serde_json::json!({"event": "custom"}));
    // Plain dict WITH ts_ns — setdefault keeps the caller's value.
    emitter.emit_value(serde_json::json!({"event": "custom", "ts_ns": 42}));
    emitter.flush(2.0);
    let sink = sink.lock().unwrap();
    assert_eq!(sink.len(), 2);
    assert!(sink[0]["ts_ns"].as_i64().unwrap() > 0);
    assert_eq!(sink[1]["ts_ns"], 42);
    emitter.close();
}

#[test]
fn emit_into_disabled_singleton_is_a_noop_until_subscribed() {
    reset_emitter_for_tests(None);
    let emitter = get_emitter();
    // Collection is opt-in: the singleton starts disabled.
    emitter.emit(&GatewayHealthEvent::new());
    emitter.flush(0.2);
    assert_eq!(emitter.stats()["dispatched"], 0);
    reset_emitter_for_tests(None);
}

#[test]
fn telemetry_emitter_alias_and_to_dict_trait() {
    // `TelemetryEmitter = MonitoringEmitter` back-compat alias.
    let emitter: TelemetryEmitter = MonitoringEmitter::new(true);
    // Any `to_dict` payload emits through the trait seam.
    let event = CronExecutionEvent::new("failed", "j2");
    emitter.subscribe(Arc::new(|batch: &[serde_json::Value]| {
        assert_eq!(batch[0]["event"], "cron_execution");
    }));
    emitter.emit(&event as &dyn ToMonitoringDict);
    emitter.flush(2.0);
    assert_eq!(emitter.stats()["dispatched"], 1);
    emitter.close();
}
