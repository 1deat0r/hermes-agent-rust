//! Parity tests for `agent/monitoring/gateway_health.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use serde_json::{json, Value};

use hermes_agent::monitoring::emitter::{get_emitter, reset_emitter_for_tests};
use hermes_agent::monitoring::gateway_health::{
    build_gateway_health_snapshot, classify_exit_reason, classify_gateway_error,
    diagnostic_event_for_log, platform_for_subsystem, redact_gateway_message,
    source_logger_for_export, subsystem_for_logger,
};
use std::sync::{Arc, Mutex};

// The process-global emitter singleton is shared; singleton-touching tests
// are serialized behind this lock (workspace convention for global state).
static EMITTER_LOCK: Mutex<()> = Mutex::new(());

// ── classification helpers ───────────────────────────────────────────────

#[test]
fn gateway_error_classification_cascade() {
    assert_eq!(
        classify_gateway_error(Some(&json!("auth token rejected"))),
        "auth_failed"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("401 Unauthorized"))),
        "auth_failed"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("rate limit hit"))),
        "rate_limited"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("timeout waiting"))),
        "timeout"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("name resolution failed"))),
        "network_error"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("missing key"))),
        "invalid_config"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("startup problem"))),
        "startup_failed"
    );
    assert_eq!(
        classify_gateway_error(Some(&json!("fatal condition"))),
        "platform_fatal"
    );
    assert_eq!(classify_gateway_error(Some(&json!("???"))), "unknown");
    assert_eq!(classify_gateway_error(None), "unknown");
}

#[test]
fn exit_reason_reduction() {
    let value = |s: &str| Value::String(s.to_string());
    // restart wins over everything.
    assert_eq!(
        classify_exit_reason(Some(&value("sigterm")), Some(&value("running")), true).as_deref(),
        Some("restart_requested")
    );
    // No raw reason and not startup_failed -> None.
    assert_eq!(
        classify_exit_reason(None, Some(&value("stopped")), false),
        None
    );
    // Signal text.
    assert_eq!(
        classify_exit_reason(
            Some(&value("killed by SIGTERM")),
            Some(&value("stopped")),
            false
        )
        .as_deref(),
        Some("signal")
    );
    // Planned stop wording.
    assert_eq!(
        classify_exit_reason(
            Some(&value("clean shutdown")),
            Some(&value("stopped")),
            false
        )
        .as_deref(),
        Some("planned_stop")
    );
    // startup_failed falls back to the state name when unclassifiable.
    assert_eq!(
        classify_exit_reason(
            Some(&value("mystery")),
            Some(&value("startup_failed")),
            false
        )
        .as_deref(),
        Some("startup_failed")
    );
}

#[test]
fn source_logger_allowlist() {
    assert_eq!(
        source_logger_for_export(Some("gateway.platforms.telegram")).as_deref(),
        Some("gateway.platforms.telegram")
    );
    assert_eq!(
        source_logger_for_export(Some("gateway")).as_deref(),
        Some("gateway")
    );
    // Underscore/alpha segments only after the prefix.
    assert_eq!(
        source_logger_for_export(Some("gateway.my_log1")).as_deref(),
        Some("gateway.my_log1")
    );
    // Digits after the first dot are invalid upstream.
    assert_eq!(source_logger_for_export(Some("gateway.9bad")), None);
    assert_eq!(source_logger_for_export(Some("other.module")), None);
    // Length bound.
    let long = format!("gateway.{}", "a".repeat(200));
    assert_eq!(source_logger_for_export(Some(&long)), None);
    assert_eq!(source_logger_for_export(None), None);
}

#[test]
fn message_redaction_bounds_and_scrubs() {
    let out = redact_gateway_message(Some("Bearer supersecrettoken"));
    assert!(out.contains("[redacted]"), "{out}");
    assert!(!out.contains("supersecrettoken"), "{out}");
    // 500-char bound: 600 a's truncated to 500.
    let long = "a".repeat(600);
    assert_eq!(redact_gateway_message(Some(&long)).chars().count(), 500);
    // None -> "".
    assert_eq!(redact_gateway_message(None), "");
}

#[test]
fn subsystem_and_platform_derivation() {
    assert_eq!(subsystem_for_logger("gateway.relay"), "platform.relay");
    assert_eq!(subsystem_for_logger("gateway.relay.ws"), "platform.relay");
    assert_eq!(
        subsystem_for_logger("gateway.platforms.telegram"),
        "platform.telegram"
    );
    assert_eq!(
        subsystem_for_logger("gateway.platforms.telegram.send"),
        "platform.telegram"
    );
    assert_eq!(subsystem_for_logger("gateway.platforms"), "platform");
    assert_eq!(subsystem_for_logger("gateway.core"), "gateway");
    assert_eq!(subsystem_for_logger("outside"), "gateway");
    assert_eq!(
        platform_for_subsystem("platform.telegram").as_deref(),
        Some("telegram")
    );
    assert_eq!(
        platform_for_subsystem("platform.relay").as_deref(),
        Some("relay")
    );
    assert_eq!(platform_for_subsystem("gateway"), None);
}

// ── snapshot builder ─────────────────────────────────────────────────────

#[test]
fn snapshot_from_empty_runtime_reports_unknown_state() {
    let snapshot =
        build_gateway_health_snapshot(None, false, "default", "install-1", "0.21.3", "unknown");
    let names: Vec<&str> = snapshot.metrics.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"hermes.gateway.up"));
    assert!(names.contains(&"hermes.gateway.active_agents"));
    assert!(names.contains(&"hermes.gateway.busy"));
    assert!(names.contains(&"hermes.gateway.drainable"));
    assert!(names.contains(&"hermes.gateway.restart_requested"));
    assert!(names.contains(&"hermes.gateway.state"));
    // Unknown state is not running: up=0, busy=0.
    let up = snapshot
        .metrics
        .iter()
        .find(|m| m.name == "hermes.gateway.up")
        .unwrap();
    assert_eq!(up.value, 0.0);
    // The health snapshot event leads.
    let lead = snapshot.events.first().expect("health snapshot event");
    let dict = lead.to_dict();
    assert_eq!(dict["event"], "gateway_health");
    assert_eq!(dict["gateway_state"], "unknown");
    assert_eq!(dict["fatal_platform_count"], 0);
}

#[test]
fn snapshot_counts_running_and_fatal_platforms() {
    let runtime = json!({
        "gateway_state": "running",
        "active_agents": 2,
        "platforms": {
            "telegram": {"state": "connected"},
            "whatsapp": {"state": "fatal", "error_code": "401 forbidden"},
            "irc": "not-a-dict",
        },
        "pid": 4242,
    });
    let snapshot =
        build_gateway_health_snapshot(Some(&runtime), true, "prod", "inst-9", "0.21.3", "systemd");
    let health = snapshot.events.first().unwrap().to_dict();
    assert_eq!(health["gateway_state"], "running");
    assert_eq!(health["gateway_busy"], true);
    assert_eq!(health["gateway_drainable"], true);
    assert_eq!(health["platform_count"], 3);
    assert_eq!(health["fatal_platform_count"], 1);
    assert_eq!(health["pid"], 4242);

    // Per-platform pairs: two platforms with dict payloads -> 2 up + 2
    // degraded metrics; the non-dict platform payload is skipped content.
    let ups = snapshot
        .metrics
        .iter()
        .filter(|m| m.name == "hermes.platform.up")
        .count();
    assert_eq!(ups, 3);
    // The fatal platform raises a diagnostic event after the health lead.
    let diags: Vec<_> = snapshot.events[1..].iter().map(|e| e.to_dict()).collect();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0]["event"], "gateway_diagnostic");
    assert_eq!(diags[0]["name"], "platform.fatal");
    assert_eq!(diags[0]["severity"], "error");
    assert_eq!(diags[0]["platform"], "whatsapp");
    // The service.instance.id is a content-free sha256 prefix, not the raw
    // install id.
    let up = snapshot
        .metrics
        .iter()
        .find(|m| m.name == "hermes.gateway.up")
        .unwrap();
    assert!(up.attributes["service.instance.id"].starts_with("sha256:"));
    assert!(!up.attributes["service.instance.id"].contains("inst-9"));
    assert_eq!(up.attributes["hermes.supervision_mode"], "systemd");
}

#[test]
fn unknown_state_value_bounded_to_unknown() {
    let runtime = json!({"gateway_state": "made-up-state"});
    let snapshot =
        build_gateway_health_snapshot(Some(&runtime), false, "default", "i", "v", "bogus-mode");
    let state = snapshot
        .metrics
        .iter()
        .find(|m| m.name == "hermes.gateway.state")
        .unwrap();
    assert_eq!(state.attributes["hermes.gateway.state"], "unknown");
    assert_eq!(state.attributes["hermes.supervision_mode"], "unknown");
}

// ── runtime-status transition events ─────────────────────────────────────

#[test]
fn transition_emits_lifecycle_and_exit_events() {
    let _guard = EMITTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_emitter_for_tests(None);
    let sink: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink_cb = Arc::clone(&sink);
    {
        let emitter = get_emitter();
        emitter.subscribe(Arc::new(move |batch: &[Value]| {
            sink_cb.lock().unwrap().extend(batch.iter().cloned());
        }));
        let current = json!({
            "gateway_state": "stopped",
            "exit_reason": "clean shutdown",
            "restart_requested": false,
            "active_agents": 0,
            "pid": 7,
        });
        hermes_agent::monitoring::gateway_health::emit_runtime_status_transition(
            Some(&json!({"gateway_state": "running"})),
            &current,
            "prod",
            "0.21.3",
        );
        // The dispatcher is asynchronous; wait for the batch to drain.
        emitter.flush(2.0);
    }
    reset_emitter_for_tests(None);
    let sink = sink.lock().unwrap();
    let names: Vec<&str> = sink.iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["gateway.lifecycle", "gateway.exit"]);
    assert_eq!(sink[0]["old_state"], "running");
    assert_eq!(sink[0]["new_state"], "stopped");
    assert_eq!(sink[0]["profile"], "prod");
}

#[test]
fn transition_without_state_change_is_silent() {
    let _guard = EMITTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_emitter_for_tests(None);
    let sink: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink_cb = Arc::clone(&sink);
    {
        let emitter = get_emitter();
        emitter.subscribe(Arc::new(move |batch: &[Value]| {
            sink_cb.lock().unwrap().extend(batch.iter().cloned());
        }));
        hermes_agent::monitoring::gateway_health::emit_runtime_status_transition(
            Some(&json!({"gateway_state": "running"})),
            &json!({"gateway_state": "running"}),
            "default",
            "0.21.3",
        );
    }
    reset_emitter_for_tests(None);
    assert!(sink.lock().unwrap().is_empty());
}

// ── diagnostic log bridge ────────────────────────────────────────────────

#[test]
fn diagnostic_log_event_gating_and_derivation() {
    // Warning from a gateway platform logger.
    let event = diagnostic_event_for_log(
        "gateway.platforms.telegram",
        "warning",
        "send failed: 403",
        "p",
        "v",
    )
    .expect("warning from gateway logger is exported");
    assert_eq!(event.name, "gateway.log.warning");
    assert_eq!(event.subsystem, "platform.telegram");
    assert_eq!(event.platform.as_deref(), Some("telegram"));
    assert_eq!(event.error_class, "auth_failed");
    assert_eq!(event.severity, "warning");
    assert_eq!(
        event.source_logger.as_deref(),
        Some("gateway.platforms.telegram")
    );

    // INFO is below the WARNING floor.
    assert!(
        diagnostic_event_for_log("gateway", "info", "hello", "p", "v").is_none(),
        "info-level records never export"
    );
    // Non-gateway logger names are allowlisted out.
    assert!(
        diagnostic_event_for_log("other.module", "error", "boom", "p", "v").is_none(),
        "non-gateway loggers never export"
    );
}
