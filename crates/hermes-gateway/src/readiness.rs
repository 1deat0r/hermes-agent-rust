//! Bounded, non-destructive readiness probes for authenticated health
//! surfaces.
//!
//! PARITY: `gateway/readiness.py` @ b9aa928 (whole module).
//!
//! The detailed health endpoint is authenticated. Even there, probes expose
//! status and counts only: never config values, credentials, paths,
//! commands, queue payloads, or exception messages (upstream surfaces only
//! the exception *type name*, reproduced here as a short fixed label since
//! Rust errors have no Python exception class).
//!
//! Probe notes preserved from upstream:
//! - `_probe_state_db` opens the database read-only via URI so a readiness
//!   probe never competes with normal state writers, and always closes the
//!   connection (the #69678/#69567 fd-leak bug class).
//! - `_probe_disk` mirrors `shutil.disk_usage` (total/used/free from
//!   statvfs, `free` = `f_bavail`).
//! - `_probe_gateway` reads only the shared runtime-status document and
//!   treats falsy `gateway_state` as `unknown` (Python `or`-fallback).

use std::ffi::CString;
use std::path::Path;

use serde_json::{json, Map, Value};

use hermes_constants::get_hermes_home;

/// PARITY: `_DISK_DEGRADED_PERCENT` (upstream line 22).
const DISK_DEGRADED_PERCENT: f64 = 90.0;

/// PARITY: `_check` (upstream lines 25-30). `detail` is omitted when empty
/// (`if detail:`), extras follow.
fn check(status: &str, detail: Option<&str>, extra: &[(&str, Value)]) -> Value {
    let mut result = Map::new();
    result.insert("status".into(), json!(status));
    if let Some(detail) = detail {
        if !detail.is_empty() {
            result.insert("detail".into(), json!(detail));
        }
    }
    for (key, value) in extra {
        result.insert((*key).into(), value.clone());
    }
    Value::Object(result)
}

/// PARITY: `_probe_state_db` (upstream lines 33-55).
fn probe_state_db(home: &Path) -> Value {
    let path = home.join("state.db");
    if !path.exists() {
        return check("ok", Some("not initialized"), &[]);
    }
    // A readiness probe must never compete with normal state writers: the
    // connection is read-only (URI `?mode=ro`), the schema query is
    // bounded, and the connection is always closed.
    let probe = (|| -> rusqlite::Result<()> {
        let uri = format!("file:{}?mode=ro", path.to_string_lossy());
        let conn = rusqlite::Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_URI | rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        conn.busy_timeout(std::time::Duration::from_secs_f64(1.0))?;
        conn.pragma_update(None, "query_only", "ON")?;
        let _ = conn.query_row("SELECT name FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, Option<String>>(0)
        })?;
        Ok(())
    })();
    match probe {
        Ok(()) => check("ok", None, &[]),
        // `type(exc).__name__` — a short class label, never the message.
        Err(_) => check("degraded", Some("sqlite_error"), &[]),
    }
}

/// PARITY: `_probe_config` (upstream lines 58-72).
fn probe_config(home: &Path) -> Value {
    let path = home.join("config.yaml");
    if !path.exists() {
        return check("ok", Some("using defaults"), &[]);
    }
    let parsed = std::fs::read_to_string(&path)
        .map_err(|_| "io_error")
        .and_then(|raw| serde_yaml::from_str::<Value>(&raw).map_err(|_| "yaml_error"));
    match parsed {
        // `raw is not None and not isinstance(raw, dict)` — an empty
        // document parses as None and stays "ok" (using defaults).
        Err(label) => check("degraded", Some(&format!("invalid config ({label})")), &[]),
        Ok(value) if !value.is_null() && !value.is_object() => {
            check("degraded", Some("top level is not a mapping"), &[])
        }
        Ok(_) => check("ok", None, &[]),
    }
}

/// `shutil.disk_usage` equivalent over `statvfs(2)`.
fn disk_usage(path: &Path) -> Option<(u64, u64, u64)> {
    let cpath = CString::new(path.as_os_str().as_encoded_bytes().to_vec()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(cpath.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    let frsize = if stat.f_frsize > 0 {
        stat.f_frsize
    } else {
        stat.f_bsize
    } as u64;
    let total = stat.f_blocks as u64 * frsize;
    let free = stat.f_bavail as u64 * frsize;
    let used = total.saturating_sub(free);
    Some((total, used, free))
}

/// PARITY: `_probe_disk` (upstream lines 75-88). Pure percentage logic is
/// in [`disk_used_percent_label`] so the degraded threshold is testable.
fn probe_disk(home: &Path) -> Value {
    let Some((total, used, free)) = disk_usage(home) else {
        return check("degraded", Some("os_error"), &[]);
    };
    let used_pct = if total > 0 {
        (used as f64 / total as f64 * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    let status = if used_pct >= DISK_DEGRADED_PERCENT {
        "degraded"
    } else {
        "ok"
    };
    check(
        status,
        None,
        &[
            ("used_percent", json!(used_pct)),
            ("free_bytes", json!(free)),
        ],
    )
}

/// PARITY: `_probe_gateway` (upstream lines 91-108). Falsy
/// `gateway_state` (missing/null/empty) becomes `unknown`; a platform
/// counts as connected when its `state` or `status` (state wins via the
/// Python `or` chain) lowercases into {connected, running, ok}.
fn probe_gateway(runtime_status: &Map<String, Value>) -> Value {
    let state = runtime_status
        .get("gateway_state")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");
    let mut connected = 0u64;
    let mut configured = 0u64;
    if let Some(platforms) = runtime_status.get("platforms").and_then(Value::as_object) {
        configured = platforms.len() as u64;
        for value in platforms.values() {
            let Some(value) = value.as_object() else {
                continue;
            };
            let label = value
                .get("state")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    value
                        .get("status")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or("")
                .to_lowercase();
            if matches!(label.as_str(), "connected" | "running" | "ok") {
                connected += 1;
            }
        }
    }
    let status = if matches!(state, "running" | "draining") {
        "ok"
    } else {
        "degraded"
    };
    check(
        status,
        None,
        &[
            ("state", json!(state)),
            ("connected_platforms", json!(connected)),
            ("platforms", json!(configured)),
        ],
    )
}

/// Bounded readiness input, mirroring `collect_runtime_readiness`'s keyword
/// arguments (queue counters default to 0).
///
/// PARITY: the `collect_runtime_readiness` signature (upstream lines
/// 111-120).
#[derive(Debug, Clone)]
pub struct RuntimeReadinessInput<'a> {
    pub configured_model: &'a str,
    pub runtime_status: Option<&'a Map<String, Value>>,
    pub active_api_runs: i64,
    pub process_completion_queue_depth: i64,
    pub active_delegations: i64,
}

impl<'a> RuntimeReadinessInput<'a> {
    pub fn new(configured_model: &'a str, runtime_status: Option<&'a Map<String, Value>>) -> Self {
        Self {
            configured_model,
            runtime_status,
            active_api_runs: 0,
            process_completion_queue_depth: 0,
            active_delegations: 0,
        }
    }
}

/// Return bounded readiness diagnostics without mutating runtime state.
///
/// PARITY: `collect_runtime_readiness` (upstream lines 111-151). The
/// overall status is "ok" only when every check reports "ok".
pub fn collect_runtime_readiness(input: RuntimeReadinessInput<'_>) -> Value {
    let home = get_hermes_home();
    let runtime: &Map<String, Value> = match input.runtime_status {
        Some(runtime) => runtime,
        // `runtime_status if isinstance(runtime_status, dict) else {}` — a
        // non-object document degrades to empty, not an error (the caller's
        // `as_object` conversion already collapsed non-objects to `None`).
        _ => &Map::new(),
    };
    let model_ok = !input.configured_model.trim().is_empty();
    let checks = json!({
        "state_db": probe_state_db(&home),
        "config": probe_config(&home),
        "model": check(if model_ok { "ok" } else { "degraded" }, None, &[]),
        "disk": probe_disk(&home),
        "gateway": probe_gateway(runtime),
        "background_queues": check(
            "ok",
            None,
            &[
                (
                    "active_api_runs",
                    json!(input.active_api_runs.max(0)),
                ),
                (
                    "process_completions",
                    json!(input.process_completion_queue_depth.max(0)),
                ),
                (
                    "active_delegations",
                    json!(input.active_delegations.max(0)),
                ),
            ],
        ),
    });
    let overall = if checks
        .as_object()
        .unwrap()
        .values()
        .all(|item| item.get("status").and_then(Value::as_str) == Some("ok"))
    {
        "ok"
    } else {
        "degraded"
    };
    json!({"status": overall, "checks": checks})
}
