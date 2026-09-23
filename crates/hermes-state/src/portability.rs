//! Session listing/rich rows, export, and import (portability).
//!
//! PARITY: hermes_state_portability.py @ 5d59366 (whole module) plus the
//! portability-mixin dependencies homed elsewhere in the god-file era:
//!   _is_explicit_fork_child_row / _is_compression_child_row,
//!   get_compression_lineage, search_sessions,
//!   _workspace_key_clause / _cwd_prefix_clause.
//!
//! PORT SEAMS (documented divergences):
//! - Reads go through `writer_conn` (a `RefCell` borrow) rather than
//!   upstream's `_read_ctx` WAL-reader pool: the Pattern-C gate
//!   (`test_no_locked_readers_gate`) flags Python methods that open
//!   `with self._lock:` for pure reads. The Rust port has no per-method
//!   writer mutex — callers already serialize `SessionDB` access — so
//!   there is no convoy analog to test; reads never take a write
//!   transaction (`BEGIN IMMEDIATE` lives only in `execute_write`).
//! - `adopt_session_lineage_from`'s donor-parameter replaces Python's
//!   duck-typed second DB; the TOCTOU export-patch oracle case
//!   (`test_donor_growth_between_export_and_retire_blocks_retirement`) is
//!   skipped — it monkeypatches the donor's instance method mid-call,
//!   which would need a trait split here (the export-time divergence
//!   guard itself is covered patch-free by `divergent_donor_is_not_retired`).
//! - The gateway-handler half of the adoption oracle (session.resume
//!   JSON-RPC) belongs to the tui_gateway row; only the unit-level
//!   `stores`-fixture tests are mirrored here.
//! - `iso_utc` reproduces Python `datetime.isoformat()` (µs precision,
//!   fraction omitted at whole seconds).

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::{json, Map, Value};

use hermes_time::{coerce_epoch, EpochInput};

use crate::common;
use crate::schema;
use crate::state::now;

use super::crud::{MessageInput, SessionRow, CONTENT_JSON_PREFIX};
use super::state::{SessionDB, WriteError};

/// The ONE session-id mint (upstream `hermes_state_ids.new_session_id`) —
/// re-exported so this module satisfies the ids-oracle SITES contract.
pub use crate::ids::new_session_id;

/// Import session text columns (upstream `_IMPORT_SESSION_TEXT_FIELDS`,
/// lines 23–26); `parent_session_id` + `source` are handled separately.
const IMPORT_SESSION_TEXT_FIELDS: &[&str] = &[
    "source",
    "user_id",
    "model",
    "system_prompt",
    "end_reason",
    "cwd",
    "git_branch",
    "git_repo_root",
    "billing_provider",
    "billing_base_url",
    "billing_mode",
    "cost_status",
    "cost_source",
    "pricing_version",
    "title",
];

/// Import message text columns (upstream `_IMPORT_MESSAGE_TEXT_FIELDS`,
/// lines 28–31); `role` is validated separately (non-empty string).
const IMPORT_MESSAGE_TEXT_FIELDS: &[&str] = &[
    "tool_call_id",
    "tool_name",
    "effect_disposition",
    "finish_reason",
    "reasoning",
    "reasoning_content",
    "platform_message_id",
    "message_id",
];

/// Size/shape accumulators for one import call (upstream `totals`).
#[derive(Default)]
struct ImportTotals {
    messages: usize,
    bytes: usize,
}

/// Map a JSON timestamp cell into [`EpochInput`] (Null/missing → Empty).
fn epoch_input(value: Option<&Value>) -> EpochInput<'_> {
    match value {
        None | Some(Value::Null) => EpochInput::Empty,
        Some(Value::Number(n)) => EpochInput::Number(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) if s.is_empty() => EpochInput::Empty,
        Some(Value::String(s)) => EpochInput::Text(s),
        Some(_) => EpochInput::Empty,
    }
}

/// Python `round()` — half-to-even on `.5` ties (gaps/wall use
/// `int(round(x))`; Python rounds ties-to-even for floats).
fn py_round_i64(v: f64) -> i64 {
    let floor = v.floor();
    let frac = v - floor;
    let f = floor as i64;
    if frac > 0.5 {
        f + 1
    } else if frac < 0.5 || (frac == 0.5 && f % 2 == 0) {
        f
    } else {
        f + 1
    }
}

/// `datetime.fromtimestamp(ts, tz=utc).isoformat()` — microseconds, the
/// fraction omitted when the stamp is whole.
fn iso_utc(ts: f64) -> String {
    let secs = ts.floor();
    let micros = (((ts - secs) * 1_000_000.0).round() as u32).min(999_999);
    let Some(dt) = chrono::DateTime::from_timestamp(secs as i64, micros * 1_000) else {
        return String::new();
    };
    if micros == 0 {
        dt.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%S.%6f+00:00").to_string()
    }
}

/// `msg.get("role") or "unknown"` — falsy cells (None/""/0/[]/{}/false)
/// become "unknown"; truthy scalars stringify Python-style.
fn timing_role(msg: &Value) -> String {
    match msg.get("role") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Number(n)) if n.as_f64() != Some(0.0) => n.to_string(),
        Some(Value::Bool(true)) => "True".to_string(),
        Some(Value::Array(a)) if !a.is_empty() => Value::Array(a.clone()).to_string(),
        Some(Value::Object(o)) if !o.is_empty() => Value::Object(o.clone()).to_string(),
        // Falsy cells (Python `x or "unknown"`); container roles never
        // occur, and non-string scalars render Python-style above.
        _ => "unknown".to_string(),
    }
}

/// Truthy gate for `tool_calls` in the timings count: non-empty arrays
/// contribute `len`, truthy non-arrays contribute 1, falsy contribute 0
/// (upstream `sum(len(tc) if isinstance(tc, list) else 1 ... if tc)`).
fn tool_calls_contrib(msg: &Value) -> usize {
    match msg.get("tool_calls") {
        None | Some(Value::Null) => 0,
        Some(Value::Array(a)) => a.len(),
        Some(Value::String(s)) if s.is_empty() => 0,
        Some(Value::Object(o)) if o.is_empty() => 0,
        Some(Value::Number(n)) if n.as_f64() == Some(0.0) => 0,
        Some(Value::Bool(false)) => 0,
        Some(_) => 1,
    }
}

/// Text-free timing evidence for a session export (ids, roles, counts and
/// durations only — never prompt text, arguments or results). Corrupt
/// timestamp cells go through `coerce_epoch`: they count as `missing` and
/// never abort the export.
///
/// PARITY: `_export_timings` (upstream lines 84–121).
pub fn export_timings(messages: &[Value], session_id: Option<&str>) -> Value {
    let mut timestamped: Vec<(usize, f64)> = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        let ts = coerce_epoch(epoch_input(msg.get("timestamp")), session_id, "timestamp");
        if let Some(ts) = ts {
            timestamped.push((i, ts));
        }
    }
    // First-seen role order (dict insertion) — serde_json preserve_order.
    let mut role_counts: Map<String, Value> = Map::new();
    for msg in messages {
        let role = timing_role(msg);
        let entry = role_counts.entry(role).or_insert_with(|| json!(0));
        *entry = json!(entry.as_i64().unwrap_or(0) + 1);
    }
    let tool_calls_emitted: usize = messages.iter().map(tool_calls_contrib).sum();

    let mut intervals: Vec<Value> = Vec::new();
    for pair in timestamped.windows(2) {
        let (pi, prev_ts) = pair[0];
        let (ni, next_ts) = pair[1];
        let prev = &messages[pi];
        let next = &messages[ni];
        intervals.push(json!({
            "from_message_id": prev.get("id").cloned().unwrap_or(Value::Null),
            "to_message_id": next.get("id").cloned().unwrap_or(Value::Null),
            "from_role": prev.get("role").cloned().unwrap_or(Value::Null),
            "to_role": next.get("role").cloned().unwrap_or(Value::Null),
            // `max(0, int(round((next - prev) * 1000)))` — py-round, floored at 0.
            "gap_ms": py_round_i64((next_ts - prev_ts) * 1000.0).max(0),
        }));
    }

    let n = timestamped.len();
    let first_ts = timestamped.first().map(|(_, t)| *t);
    let last_ts = timestamped.last().map(|(_, t)| *t);
    let wall = first_ts
        .zip(last_ts)
        .map(|(f, l)| py_round_i64((l - f) * 1000.0).max(0));
    let largest_gap = intervals.iter().filter_map(|i| i["gap_ms"].as_i64()).max();
    let tool_result_count = role_counts.get("tool").and_then(Value::as_i64).unwrap_or(0);

    json!({
        "source": "message_timestamps",
        "available": n > 0,
        "complete": false,
        "unavailable_reason": if n > 0 { Value::Null } else { json!("no_timestamped_messages") },
        "message_timestamps": {"available": n, "missing": messages.len() - n},
        "first_message_at": first_ts.map(iso_utc).map(Value::String).unwrap_or(Value::Null),
        "last_message_at": last_ts.map(iso_utc).map(Value::String).unwrap_or(Value::Null),
        "wall_clock_ms": wall.map(Value::from).unwrap_or(Value::Null),
        "largest_gap_ms": largest_gap.map(Value::from).unwrap_or(Value::Null),
        "role_counts": Value::Object(role_counts),
        "tool_result_count": tool_result_count,
        "tool_calls_emitted": tool_calls_emitted,
        "intervals": Value::Array(intervals),
    })
}

/// Import size limits (upstream class constants 2171–2175).
const IMPORT_MAX_SESSIONS: usize = 500;
const IMPORT_MAX_MESSAGES_PER_SESSION: usize = 10_000;
const IMPORT_MAX_TOTAL_MESSAGES: usize = 50_000;
const IMPORT_MAX_SESSION_BYTES: usize = 5 * 1024 * 1024;
const IMPORT_MAX_TOTAL_BYTES: usize = 25 * 1024 * 1024;

/// IN-list chunk for the enriched multi-row fetch (SQLite's 999 bound cap).
const RICH_ROWS_CHUNK: usize = 900;

static COMPACT_COLS: OnceLock<String> = OnceLock::new();

/// Column list for `compact_rows`: every sessions column except prompt
/// storage internals, aliased with the `s.` prefix.
// PARITY: hermes_state_portability.py SessionPortabilityMixin._compact_session_cols
pub(crate) fn compact_session_cols() -> String {
    COMPACT_COLS
        .get_or_init(|| {
            let declared = schema::parse_schema_columns(crate::common::SCHEMA_SQL);
            let sessions = declared
                .iter()
                .find(|(name, _)| name == "sessions")
                .map(|(_, cols)| cols)
                .cloned()
                .unwrap_or_default();
            sessions
                .iter()
                .filter(|(name, _)| name != "system_prompt" && name != "system_prompt_hash")
                .map(|(name, _)| format!("s.{}", name))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .clone()
}

/// Render any row as a JSON object via its statement's column metadata
/// (Null → null, Integer → number, Real → number, Text → string, Blob →
/// lossy string). Mirrors Python's `dict(row)` for sqlite3.Row.
pub(crate) fn row_to_value(row: &Row<'_>) -> rusqlite::Result<Value> {
    let stmt = row.as_ref();
    let mut obj = serde_json::Map::new();
    for i in 0..stmt.column_count() {
        let name = stmt
            .column_name(i)
            .map(str::to_string)
            .unwrap_or_else(|_| format!("c{}", i));
        match row.get_ref(i)? {
            rusqlite::types::ValueRef::Null => {
                obj.insert(name, Value::Null);
            }
            rusqlite::types::ValueRef::Integer(n) => {
                obj.insert(name, json!(n));
            }
            rusqlite::types::ValueRef::Real(f) => {
                obj.insert(name, json!(f));
            }
            rusqlite::types::ValueRef::Text(t) => {
                obj.insert(name, Value::String(String::from_utf8_lossy(t).into_owned()));
            }
            rusqlite::types::ValueRef::Blob(b) => {
                obj.insert(name, Value::String(String::from_utf8_lossy(b).into_owned()));
            }
        }
    }
    Ok(Value::Object(obj))
}

/// Messages row as `get_messages` returns it upstream (decoded content,
/// tool_calls JSON parsed with [] fallback, display_metadata decoded;
/// observed/active/compacted stay raw 0/1 ints).
pub(crate) fn message_row_to_value(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut v = row_to_value(row)?;
    if let Some(obj) = v.as_object_mut() {
        // _decode_content
        if let Some(raw) = obj.get("content").cloned() {
            let decoded = match raw {
                Value::String(s) if s.starts_with(CONTENT_JSON_PREFIX) => {
                    serde_json::from_str::<Value>(&s[CONTENT_JSON_PREFIX.len()..])
                        .unwrap_or(Value::String(s))
                }
                other => other,
            };
            obj.insert("content".to_string(), decoded);
        }
        // tool_calls JSON with [] fallback (upstream `if msg.get(...)` is
        // falsy for NULL/empty strings, which then stay as their raw value)
        match obj.get("tool_calls").and_then(|v| v.as_str()) {
            Some(raw) if !raw.is_empty() => {
                let parsed = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!([]));
                obj.insert("tool_calls".to_string(), parsed);
            }
            Some("") => {
                obj.insert("tool_calls".to_string(), Value::String(String::new()));
            }
            _ => {
                obj.insert("tool_calls".to_string(), Value::Null);
            }
        }
        // display_metadata decode (double-encoded tolerance)
        if let Some(Value::String(s)) = obj.get("display_metadata").cloned() {
            let meta = serde_json::from_str::<Value>(&s).ok();
            let meta = match meta {
                Some(Value::String(s2)) => serde_json::from_str::<Value>(&s2).ok(),
                other => other,
            };
            match meta {
                Some(Value::Object(_)) => {
                    obj.insert("display_metadata".to_string(), meta.unwrap());
                }
                _ => {
                    obj.insert("display_metadata".to_string(), Value::Null);
                }
            }
        } else {
            obj.insert("display_metadata".to_string(), Value::Null);
        }
    }
    Ok(v)
}

fn value_text(v: Option<&Value>) -> Option<String> {
    v.and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    })
}

fn value_str_or_default(v: Option<&Value>, default: &str) -> String {
    value_text(v).unwrap_or_else(|| default.to_string())
}

struct RichRowEnvelope {
    _preview_raw: String,
    _row: Value,
}

fn shape_envelope(row: &Row<'_>) -> rusqlite::Result<RichRowEnvelope> {
    let v = row_to_value(row)?;
    let preview_raw = v
        .get("_preview_raw")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    Ok(RichRowEnvelope {
        _preview_raw: preview_raw,
        _row: v,
    })
}

fn enrich(s: Value, preview_raw: &str) -> Value {
    let mut s = s;
    if let Some(obj) = s.as_object_mut() {
        obj.remove("_preview_raw");
        obj.insert(
            "preview".to_string(),
            Value::String(common::_shape_preview(preview_raw.to_string())),
        );
    }
    s
}

/// Parse a session dict's `model_config` for the fork-marker test.
fn model_config_fork_markers(model_config: Option<&str>) -> (bool, bool) {
    let Some(raw) = model_config else {
        return (false, false);
    };
    let Ok(cfg) = serde_json::from_str::<Value>(raw) else {
        return (false, false);
    };
    let obj = match cfg {
        Value::Object(o) => o,
        _ => return (false, false),
    };
    (
        obj.get("_branched_from").is_some_and(|v| !v.is_null()),
        obj.get("_delegate_from").is_some_and(|v| !v.is_null()),
    )
}

/// `_is_explicit_fork_child_row` @ b9aa928: tool source or branched/delegate
/// marker in model_config.
pub(crate) fn is_explicit_fork_child_row(session: &SessionRow) -> bool {
    if session.source == "tool" {
        return true;
    }
    let (branched, delegated) = model_config_fork_markers(session.model_config.as_deref());
    branched || delegated
}

/// `_is_compression_child_row` @ b9aa928: parent exists, not an explicit
/// fork, and the parent ended with end_reason='compression'.
fn is_compression_child_row(db: &SessionDB, child: &SessionRow) -> Result<bool, WriteError> {
    let Some(parent_id) = child.parent_session_id.as_deref() else {
        return Ok(false);
    };
    if is_explicit_fork_child_row(child) {
        return Ok(false);
    }
    let parent = db.get_session(parent_id)?;
    Ok(parent.is_some_and(|p| p.end_reason.as_deref() == Some("compression")))
}

pub struct ImportResult {
    pub ok: bool,
    pub imported: usize,
    pub skipped: usize,
    pub detached: usize,
    pub imported_ids: Vec<String>,
    pub skipped_ids: Vec<String>,
    pub errors: Vec<Value>,
}

pub fn import_result_value(r: &ImportResult) -> Value {
    json!({
        "ok": r.ok,
        "imported": r.imported,
        "skipped": r.skipped,
        "detached": r.detached,
        "imported_ids": r.imported_ids,
        "skipped_ids": r.skipped_ids,
        "errors": r.errors,
    })
}

impl SessionDB {
    /// `_is_explicit_fork_child_row` and `_is_compression_child_row` public
    /// wrappers (lineage surface).
    pub fn is_explicit_fork_child_row(&self, session: &SessionRow) -> bool {
        is_explicit_fork_child_row(session)
    }

    /// Distinct non-empty session cwds with usage stats, for repo discovery.
    /// PARITY: hermes_state_portability.py distinct_session_cwds
    pub fn distinct_session_cwds(&self, include_archived: bool) -> Result<Vec<Value>, WriteError> {
        let mut where_sql = "cwd IS NOT NULL AND TRIM(cwd) != ''".to_string();
        if !include_archived {
            where_sql += " AND archived = 0";
        }
        let sql = format!(
            "SELECT cwd AS cwd, COUNT(*) AS sessions, \
             MAX(COALESCE(ended_at, started_at, 0)) AS last_active \
             FROM sessions WHERE {} GROUP BY cwd",
            where_sql
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, f64>(2)?,
                ))
            })
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows
            .into_iter()
            .map(|(cwd, sessions, last_active)| {
                json!({"cwd": cwd, "sessions": sessions, "last_active": last_active})
            })
            .collect())
    }

    /// List the run sessions produced by a single cron job, newest first.
    /// PARITY: hermes_state_portability.py list_cron_job_runs
    pub fn list_cron_job_runs(
        &self,
        job_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Value>, WriteError> {
        let prefix = format!("cron_{}_", job_id);
        // Half-open upper bound for an index range scan: increment the
        // final byte of the prefix so the range covers exactly the ids that
        // start with `prefix` (prefix always ends in '_').
        let mut bytes = prefix.clone().into_bytes();
        let last = bytes.last_mut().unwrap();
        *last = last.saturating_add(1);
        let prefix_hi = String::from_utf8_lossy(&bytes).into_owned();

        let sql = format!(
            "SELECT s.*, \
                COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved, \
                COALESCE(\n        (SELECT {}\n         FROM messages m\n         WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL\n         ORDER BY m.timestamp, m.id LIMIT 1),\n        ''\n    ) AS _preview_raw, \
                {} AS last_active \
             FROM sessions s \
             LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
             WHERE s.source = 'cron' AND s.id >= ? AND s.id < ? \
             ORDER BY s.started_at DESC, s.id DESC \
             LIMIT ? OFFSET ?",
            common::_preview_raw_select(),
            common::_sql_session_last_active("s"),
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params![prefix, prefix_hi, limit, offset],
                shape_envelope,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows
            .into_iter()
            .map(|e| {
                let s = super::crud::fold_session_dict(e._row);
                enrich(s, &e._preview_raw)
            })
            .collect())
    }

    /// Fetch multiple sessions with the enriched columns in one query.
    /// PARITY: hermes_state_portability.py _get_session_rich_rows_batch
    pub fn get_session_rich_rows_batch(
        &self,
        session_ids: &[String],
        compact_rows: bool,
    ) -> Result<HashMap<String, Value>, WriteError> {
        let ids: Vec<String> = session_ids
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        if ids.len() > RICH_ROWS_CHUNK {
            let mut result = HashMap::new();
            for chunk in ids.chunks(RICH_ROWS_CHUNK) {
                result.extend(self.get_session_rich_rows_batch(chunk, compact_rows)?);
            }
            return Ok(result);
        }
        // Upstream flushes queued token counts first (same read-your-writes
        // guarantee as list_sessions_rich). The token-writer port is not yet
        // landed, so the queue is always empty — no-op seam.
        let sel = if compact_rows {
            compact_session_cols()
        } else {
            "s.*".to_string()
        };
        let placeholders = vec!["?"; ids.len()].join(",");
        let prompt_select = if compact_rows {
            ""
        } else {
            ", COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved"
        };
        let prompt_join = if compact_rows {
            ""
        } else {
            "LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash"
        };
        let sql = format!(
            "SELECT {}{},\n    COALESCE(\n        (SELECT {}\n         FROM messages m\n         WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL\n         ORDER BY m.timestamp, m.id LIMIT 1),\n        ''\n    ) AS _preview_raw,\n    {} AS last_active\nFROM sessions s\n{}\nWHERE s.id IN ({})",
            sel,
            prompt_select,
            common::_preview_raw_select(),
            common::_sql_session_last_active("s"),
            prompt_join,
            placeholders,
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ids.iter()), shape_envelope)
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        let mut result = HashMap::new();
        for e in rows {
            let id = e
                ._row
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let s = if compact_rows {
                e._row
            } else {
                super::crud::fold_session_dict(e._row)
            };
            result.insert(id, enrich(s, &e._preview_raw));
        }
        Ok(result)
    }

    /// Fetch a single session with the enriched columns. Public wrapper.
    /// PARITY: hermes_state_portability.py get_session_rich_row
    pub fn get_session_rich_row(
        &self,
        session_id: &str,
        compact_rows: bool,
    ) -> Result<Option<Value>, WriteError> {
        Ok(self
            .get_session_rich_rows_batch(&[session_id.to_string()], compact_rows)?
            .remove(session_id))
    }

    /// Titled sessions whose first user turn was a `/skill` invocation.
    /// PARITY: hermes_state_portability.py list_skill_scaffolded_sessions
    pub fn list_skill_scaffolded_sessions(&self, limit: i64) -> Result<Vec<Value>, WriteError> {
        let sql = "SELECT s.id, s.title, m.content \
                   FROM sessions s \
                   JOIN messages m ON m.id = (\
                       SELECT m2.id FROM messages m2 \
                       WHERE m2.session_id = s.id AND m2.role = 'user' \
                         AND m2.content IS NOT NULL \
                       ORDER BY m2.timestamp, m2.id LIMIT 1\
                   ) \
                   WHERE s.title IS NOT NULL AND m.content LIKE ? \
                   ORDER BY s.started_at DESC \
                   LIMIT ?";
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params![crate::skill::SKILL_SCAFFOLD_SQL_LIKE, limit],
                row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }

    /// The session's first assistant reply as plain text ('' when none).
    /// PARITY: hermes_state_portability.py get_first_assistant_text
    pub fn get_first_assistant_text(&self, session_id: &str) -> Result<String, WriteError> {
        let conn = self.writer_conn();
        let raw: Option<rusqlite::types::Value> = conn
            .query_row(
                "SELECT content FROM messages \
                 WHERE session_id = ? AND role = 'assistant' AND content IS NOT NULL \
                 ORDER BY timestamp, id LIMIT 1",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(match raw {
            Some(content) => match super::crud::decode_content(Some(content)) {
                Some(Value::String(s)) => s,
                _ => String::new(),
            },
            None => String::new(),
        })
    }

    /// Export a single session with all its messages + timings as a dict.
    /// PARITY: hermes_state_portability.py `_with_messages` / export_session
    pub fn export_session(&self, session_id: &str) -> Result<Option<Value>, WriteError> {
        let Some(mut session) = self.get_session_dict(session_id)? else {
            return Ok(None);
        };
        let messages = self.get_messages_dicts(session_id, false, None, 0)?;
        if let Some(obj) = session.as_object_mut() {
            let msgs = Value::Array(messages);
            let timings = export_timings(msgs.as_array().unwrap(), Some(session_id));
            obj.insert("messages".to_string(), msgs);
            obj.insert("timings".to_string(), timings);
        }
        Ok(Some(session))
    }

    /// Export a compression lineage as one logical session dict.
    /// PARITY: hermes_state_portability.py export_session_lineage
    pub fn export_session_lineage(&self, session_id: &str) -> Result<Option<Value>, WriteError> {
        let lineage_ids = self.get_compression_lineage(session_id)?;
        if lineage_ids.is_empty() {
            return Ok(None);
        }
        let mut segments = Vec::new();
        for sid in &lineage_ids {
            if let Some(segment) = self.export_session(sid)? {
                segments.push(segment);
            }
        }
        if segments.is_empty() {
            return Ok(None);
        }
        let mut base = segments.last().unwrap().clone();
        let base_obj = base.as_object_mut().unwrap();
        let total_messages: usize = segments
            .iter()
            .map(|seg| {
                seg.get("messages")
                    .and_then(|m| m.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0)
            })
            .sum();
        let lineage_session_ids: Vec<Value> = segments
            .iter()
            .filter_map(|seg| seg.get("id").cloned())
            .collect();
        let all_messages: Vec<Value> = segments
            .iter()
            .flat_map(|seg| {
                seg.get("messages")
                    .and_then(|m| m.as_array().cloned())
                    .unwrap_or_default()
            })
            .collect();
        base_obj.insert("segments".to_string(), Value::Array(segments));
        base_obj.insert(
            "lineage_session_ids".to_string(),
            Value::Array(lineage_session_ids),
        );
        base_obj.insert("message_count".to_string(), json!(total_messages));
        base_obj.insert("messages".to_string(), Value::Array(all_messages.clone()));
        // Merged timings span EVERY segment's messages (the segment-level
        // copies inside `segments` keep their own).
        let timings = export_timings(&all_messages, Some(session_id));
        base_obj.insert("timings".to_string(), timings);
        Ok(Some(base))
    }

    /// Export all sessions (with messages) as a list of dicts. Note: tasks
    /// like JSONL backup use this; upstream calls search_sessions(limit=100000).
    /// PARITY: hermes_state_portability.py export_all
    pub fn export_all(&self, source: Option<&str>) -> Result<Vec<Value>, WriteError> {
        let sessions = self.search_sessions(source, 100_000, 0, None)?;
        let mut results = Vec::new();
        for session in &sessions {
            let sid = session
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut s = session.clone();
            let messages = self.get_messages_dicts(&sid, false, None, 0)?;
            if let Some(obj) = s.as_object_mut() {
                let msgs = Value::Array(messages);
                let timings = export_timings(msgs.as_array().unwrap(), Some(&sid));
                obj.insert("messages".to_string(), msgs);
                obj.insert("timings".to_string(), timings);
            }
            results.push(s);
        }
        Ok(results)
    }

    /// Compression ancestors through tip in chronological order.
    /// PARITY: hermes_state.py SessionDB.get_compression_lineage @ b9aa928
    pub fn get_compression_lineage(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let Some(mut root) = self.get_session(session_id)? else {
            return Ok(vec![]);
        };
        if is_explicit_fork_child_row(&root) {
            return Ok(vec![session_id.to_string()]);
        }
        let mut ancestors = std::collections::HashSet::new();
        ancestors.insert(root.id.clone());
        while is_compression_child_row(self, &root)? {
            let Some(parent_id) = root.parent_session_id.clone() else {
                break;
            };
            let Some(parent) = self.get_session(&parent_id)? else {
                break;
            };
            if ancestors.contains(&parent.id) {
                break;
            }
            root = parent;
            ancestors.insert(root.id.clone());
        }

        let mut lineage = vec![root.id.clone()];
        let mut seen = std::collections::HashSet::new();
        seen.insert(root.id.clone());
        let mut current = root;
        while current.end_reason.as_deref() == Some("compression") {
            let conn = self.writer_conn();
            let mut stmt = conn
                .prepare(
                    "SELECT s.*, COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
                     FROM sessions s \
                     LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
                     WHERE s.parent_session_id = ? ORDER BY s.started_at ASC",
                )
                .map_err(WriteError::Sqlite)?;
            let rows = stmt
                .query_map(rusqlite::params![current.id], super::crud::session_row)
                .map_err(WriteError::Sqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(WriteError::Sqlite)?;
            let mut next_child = None;
            for candidate in rows {
                if is_compression_child_row(self, &candidate)? {
                    next_child = Some(candidate);
                    break;
                }
            }
            let Some(child) = next_child else {
                break;
            };
            if seen.contains(&child.id) {
                break;
            }
            lineage.push(child.id.clone());
            seen.insert(child.id.clone());
            if child.id == session_id {
                // Continue to include later compression tips only when the
                // requested session itself was compacted.
            }
            current = child;
        }
        if lineage.contains(&session_id.to_string()) {
            Ok(lineage)
        } else {
            Ok(vec![session_id.to_string()])
        }
    }

    /// List sessions, optionally filtered by source, enriched with a computed
    /// `last_active` column, ordered MRU-first.
    /// PARITY: hermes_state.py SessionDB.search_sessions @ b9aa928
    pub fn search_sessions(
        &self,
        source: Option<&str>,
        limit: i64,
        offset: i64,
        workspace_key: Option<&str>,
    ) -> Result<Vec<Value>, WriteError> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(source) = source {
            where_clauses.push("s.source = ?".to_string());
            params.push(Box::new(source.to_string()));
        }
        if let Some(key) = workspace_key {
            let (clause, ws_params) = workspace_key_clause(key);
            where_clauses.push(clause);
            for p in ws_params {
                params.push(Box::new(p));
            }
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT s.*, \
                COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved, \
                {} AS last_active \
             FROM sessions s \
             LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
             {}\
             ORDER BY last_active DESC, s.started_at DESC, s.id DESC LIMIT ? OFFSET ?",
            common::_sql_session_last_active("s"),
            where_sql,
        );
        params.push(Box::new(limit));
        params.push(Box::new(offset));
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|x| x as &dyn rusqlite::ToSql)),
                |r| {
                    let v = row_to_value(r)?;
                    Ok(super::crud::fold_session_dict(v))
                },
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }

    // ── import_sessions ────────────────────────────────────────────────────

    /// Import sessions exported by export_session/export_all.
    /// PARITY: hermes_state_portability.py SessionPortabilityMixin.import_sessions
    pub fn import_sessions(&self, sessions: &[Value]) -> Result<ImportResult, WriteError> {
        if sessions.len() > IMPORT_MAX_SESSIONS {
            return Ok(ImportResult {
                ok: false,
                imported: 0,
                skipped: 0,
                detached: 0,
                imported_ids: vec![],
                skipped_ids: vec![],
                errors: vec![json!({
                    "index": 0,
                    "error": format!(
                        "sessions must contain at most {} entries",
                        IMPORT_MAX_SESSIONS
                    ),
                })],
            });
        }

        let mut normalized: Vec<(usize, Value, Vec<MessageInput>)> = Vec::new();
        let mut errors: Vec<Value> = Vec::new();
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut total_messages = 0usize;
        let mut total_bytes = 0usize;

        for (index, raw) in sessions.iter().enumerate() {
            match validate_import_entry(raw, &seen_ids, &mut total_messages, &mut total_bytes) {
                Err((session_id, message)) => {
                    errors.push(import_error(index, &session_id, &message));
                    continue;
                }
                Ok((session_id, clean_session, clean_messages)) => {
                    seen_ids.insert(session_id.clone());
                    normalized.push((index, clean_session, clean_messages));
                }
            }
        }

        if !errors.is_empty() {
            return Ok(ImportResult {
                ok: false,
                imported: 0,
                skipped: 0,
                detached: 0,
                imported_ids: vec![],
                skipped_ids: vec![],
                errors,
            });
        }

        let f = move |conn: &Connection| -> Result<ImportResult, WriteError> {
            let mut imported_ids: Vec<String> = Vec::new();
            let mut skipped_ids: Vec<String> = Vec::new();
            let mut parent_updates: Vec<(String, String)> = Vec::new();
            let mut detached = 0usize;

            for item in &normalized {
                let raw = &item.1;
                let raw_obj = raw.as_object().unwrap();
                let session_id = raw_obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let exists: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM sessions WHERE id = ? LIMIT 1",
                        rusqlite::params![session_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if exists.is_some() {
                    skipped_ids.push(session_id);
                    continue;
                }

                import_session_row(conn, &item.1, &item.2, &session_id)?;

                let parent_id = value_text(raw_obj.get("parent_session_id"))
                    .map(|p| p.trim().to_string())
                    .unwrap_or_default();
                if !parent_id.is_empty() {
                    parent_updates.push((session_id.clone(), parent_id));
                }
                imported_ids.push(session_id);
            }

            let mut parent_by_child: HashMap<String, String> =
                parent_updates.iter().cloned().collect();

            fn would_create_cycle(
                conn: &Connection,
                session_id: &str,
                parent_id: &str,
                parent_by_child: &HashMap<String, String>,
            ) -> Result<bool, WriteError> {
                let mut seen = std::collections::HashSet::new();
                seen.insert(session_id.to_string());
                let mut current = parent_id.to_string();
                loop {
                    if seen.contains(&current) {
                        return Ok(true);
                    }
                    seen.insert(current.clone());
                    if let Some(grandparent) = parent_by_child.get(&current) {
                        current = grandparent.clone();
                        continue;
                    }
                    let row: Option<Option<String>> = conn
                        .query_row(
                            "SELECT parent_session_id FROM sessions WHERE id = ? LIMIT 1",
                            rusqlite::params![current],
                            |r| r.get(0),
                        )
                        .optional()?;
                    let row = row.flatten();
                    match row {
                        Some(next) => {
                            current = next;
                        }
                        None => return Ok(false),
                    }
                }
            }

            for (session_id, parent_id) in &parent_updates {
                let parent_exists: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM sessions WHERE id = ? LIMIT 1",
                        rusqlite::params![parent_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if parent_exists.is_some()
                    && !would_create_cycle(conn, session_id, parent_id, &parent_by_child)?
                {
                    conn.execute(
                        "UPDATE sessions SET parent_session_id = ? WHERE id = ?",
                        rusqlite::params![parent_id, session_id],
                    )?;
                } else {
                    // Drop only the closing edge; later entries can still
                    // attach to this now-root session, preserving the
                    // acyclic portion of a malformed imported lineage.
                    parent_by_child.remove(session_id);
                    detached += 1;
                }
            }

            Ok(ImportResult {
                ok: true,
                imported: imported_ids.len(),
                skipped: skipped_ids.len(),
                detached,
                imported_ids,
                skipped_ids,
                errors: vec![],
            })
        };
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }
}

/// One payload session → (session_id, normalized session dict, messages).
/// Err carries `(session_id_for_error, message)` — the caller wraps the
/// index. Validation order mirrors upstream `_validate_import_session`
/// (bytes accumulate before their limit check — a rejected oversize entry
/// still counts; totals likewise keep a failed total-messages add).
///
/// PARITY: `_validate_import_payload` / `_validate_import_session` /
/// `_normalize_import_session` (upstream lines 459–512) folded into one
/// helper the foreign-import path shares.
fn validate_import_entry(
    raw: &Value,
    seen_ids: &std::collections::HashSet<String>,
    total_messages: &mut usize,
    total_bytes: &mut usize,
) -> Result<(String, Value, Vec<MessageInput>), (String, String)> {
    let Some(raw_obj) = raw.as_object() else {
        return Err((String::new(), "session must be an object".to_string()));
    };
    let session_id = raw_obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if session_id.is_empty() {
        return Err((String::new(), "session id is required".to_string()));
    }
    if seen_ids.contains(&session_id) {
        return Err((session_id, "duplicate session id".to_string()));
    }
    let messages_value = raw_obj
        .get("messages")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let Some(msg_arr) = messages_value.as_array() else {
        return Err((session_id, "messages must be a list".to_string()));
    };
    if msg_arr.len() > IMPORT_MAX_MESSAGES_PER_SESSION {
        return Err((
            session_id,
            "messages exceeds the per-session import limit".to_string(),
        ));
    }
    if msg_arr.iter().any(|m| !m.is_object()) {
        return Err((session_id, "messages must contain only objects".to_string()));
    }

    // Size budget: `timings` is derived at export time and rebuilt on the
    // next export — it must not eat into the content's budget (upstream
    // line 499).
    let session_bytes = match serde_json::to_string(raw) {
        Ok(full) => {
            let stripped = if raw_obj.contains_key("timings") {
                let filtered: serde_json::Map<String, Value> = raw_obj
                    .iter()
                    .filter(|(k, _)| k.as_str() != "timings")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                serde_json::to_string(&Value::Object(filtered)).unwrap_or(full)
            } else {
                full
            };
            stripped.len()
        }
        Err(_) => return Err((session_id, "session must be JSON serializable".to_string())),
    };
    if session_bytes > IMPORT_MAX_SESSION_BYTES {
        return Err((
            session_id,
            "session exceeds the import size limit".to_string(),
        ));
    }
    *total_bytes += session_bytes;
    if *total_bytes > IMPORT_MAX_TOTAL_BYTES {
        return Err((
            session_id,
            "import exceeds the total size limit".to_string(),
        ));
    }

    // Normalize (upstream `_normalize_import_session`).
    let mut clean_session = raw.clone();
    {
        let obj = clean_session.as_object_mut().expect("object checked");
        obj.insert("id".to_string(), json!(session_id));
        let mc: Result<Option<String>, String> = match obj.get("model_config").cloned() {
            None | Some(Value::Null) => Ok(None),
            Some(v) => import_json_object_or_none(v, "model_config"),
        };
        let parent: Result<Option<String>, String> = match obj.get("parent_session_id").cloned() {
            None | Some(Value::Null) => Ok(None),
            Some(v) => import_text_or_none(v, "parent_session_id"),
        };
        let mut field_results: Result<Vec<(String, Option<String>)>, String> = Ok(Vec::new());
        if mc.is_ok() && parent.is_ok() {
            for field in IMPORT_SESSION_TEXT_FIELDS {
                let v = match obj.get(*field).cloned() {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => import_text_or_none(v, field),
                };
                match v {
                    Ok(v) => field_results
                        .as_mut()
                        .expect("ok at this point")
                        .push((field.to_string(), v)),
                    Err(e) => {
                        field_results = Err(e);
                        break;
                    }
                }
            }
        }
        let (mc, parent) = match (mc, parent) {
            (Ok(mc), Ok(parent)) => (mc, parent),
            (Err(e), _) | (_, Err(e)) => return Err((session_id.clone(), e)),
        };
        let fields = match field_results {
            Ok(fields) => fields,
            Err(e) => return Err((session_id.clone(), e)),
        };
        obj.insert(
            "model_config".to_string(),
            mc.map(Value::String).unwrap_or(Value::Null),
        );
        obj.insert(
            "parent_session_id".to_string(),
            parent.map(Value::String).unwrap_or(Value::Null),
        );
        for (field, v) in fields {
            obj.insert(field, v.map(Value::String).unwrap_or(Value::Null));
        }
    }

    let mut clean_messages: Vec<MessageInput> = Vec::new();
    for (message_index, message) in msg_arr.iter().enumerate() {
        let mut clean_message = message.clone();
        let role = clean_message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if role.is_empty() {
            return Err((
                session_id.clone(),
                format!("messages[{message_index}].role must be a non-empty string"),
            ));
        }
        {
            let mo = clean_message.as_object_mut().expect("object checked");
            for field in IMPORT_MESSAGE_TEXT_FIELDS {
                let v = match mo.get(*field).cloned() {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => import_text_or_none(v, field),
                };
                match v {
                    Ok(v) => {
                        mo.insert(
                            field.to_string(),
                            v.map(Value::String).unwrap_or(Value::Null),
                        );
                    }
                    Err(e) => return Err((session_id.clone(), e)),
                }
            }
            let token_count = match mo.get("token_count").cloned() {
                None | Some(Value::Null) => Ok(None),
                Some(v) => import_int_or_none(v, "token_count"),
            };
            match token_count {
                Ok(v) => {
                    mo.insert(
                        "token_count".to_string(),
                        v.map(|n| json!(n)).unwrap_or(Value::Null),
                    );
                }
                Err(e) => return Err((session_id.clone(), e)),
            }
        }
        clean_messages.push(message_input_from_import(&clean_message));
    }

    *total_messages += clean_messages.len();
    if *total_messages > IMPORT_MAX_TOTAL_MESSAGES {
        return Err((
            session_id,
            "messages exceeds the total import limit".to_string(),
        ));
    }
    Ok((session_id, clean_session, clean_messages))
}

/// INSERT one normalized session + its messages; counts fixed up after.
///
/// PARITY: `_import_session_row` (upstream lines 514–534). `started_at`
/// goes through `coerce_epoch` (upstream) — a corrupt stamp falls back to
/// `now()`.
fn import_session_row(
    conn: &Connection,
    clean_session: &Value,
    messages: &[MessageInput],
    session_id: &str,
) -> Result<(), WriteError> {
    let raw_obj = clean_session.as_object().expect("object checked");
    let started_at = coerce_epoch(
        epoch_input(raw_obj.get("started_at")),
        Some(session_id),
        "started_at",
    )
    .unwrap_or_else(now);
    let archived = if truthy_value(raw_obj.get("archived")) {
        1
    } else {
        0
    };
    let system_prompt_hash =
        schema::store_system_prompt(conn, value_text(raw_obj.get("system_prompt")))?;

    conn.execute(
        "INSERT INTO sessions (
           id, source, user_id, model, model_config, system_prompt,
           system_prompt_hash,
           parent_session_id, started_at, ended_at, end_reason,
           message_count, tool_call_count, input_tokens, output_tokens,
           cache_read_tokens, cache_write_tokens, reasoning_tokens,
           cwd, git_branch, git_repo_root,
           billing_provider, billing_base_url, billing_mode,
           estimated_cost_usd, actual_cost_usd, cost_status, cost_source,
           pricing_version, title, api_call_count, archived
         ) VALUES (?, ?, ?, ?, ?, NULL, ?, NULL, ?, ?, ?, 0, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            session_id,
            value_str_or_default(raw_obj.get("source"), "import"),
            value_text(raw_obj.get("user_id")),
            value_text(raw_obj.get("model")),
            value_text(raw_obj.get("model_config")),
            system_prompt_hash,
            started_at,
            float_or_none(raw_obj.get("ended_at")),
            value_text(raw_obj.get("end_reason")),
            int_or_default(raw_obj.get("input_tokens"), 0),
            int_or_default(raw_obj.get("output_tokens"), 0),
            int_or_default(raw_obj.get("cache_read_tokens"), 0),
            int_or_default(raw_obj.get("cache_write_tokens"), 0),
            int_or_default(raw_obj.get("reasoning_tokens"), 0),
            value_text(raw_obj.get("cwd")),
            value_text(raw_obj.get("git_branch")),
            value_text(raw_obj.get("git_repo_root")),
            value_text(raw_obj.get("billing_provider")),
            value_text(raw_obj.get("billing_base_url")),
            value_text(raw_obj.get("billing_mode")),
            float_or_none(raw_obj.get("estimated_cost_usd")),
            float_or_none(raw_obj.get("actual_cost_usd")),
            value_text(raw_obj.get("cost_status")),
            value_text(raw_obj.get("cost_source")),
            value_text(raw_obj.get("pricing_version")),
            value_text(raw_obj.get("title")),
            int_or_default(raw_obj.get("api_call_count"), 0),
            archived,
        ],
    )?;

    // _reasoning_json_value: keep JSON strings parsed so the insert
    // serializer stores compact JSON (same as import).
    let sanitized: Vec<MessageInput> = messages
        .iter()
        .map(|m| {
            let mut m = m.clone();
            for key in [
                "reasoning_details",
                "codex_reasoning_items",
                "codex_message_items",
            ] {
                if let Some(field) = m.reasoning_field_mut(key) {
                    if let Some(v) = field.take() {
                        *field = Some(reasoning_json_value(v));
                    }
                }
            }
            m
        })
        .collect();

    let (total_messages, total_tool_calls) =
        SessionDB::insert_message_rows(conn, session_id, &sanitized)?;
    conn.execute(
        "UPDATE sessions SET message_count = ?, tool_call_count = ? WHERE id = ?",
        rusqlite::params![total_messages, total_tool_calls, session_id],
    )?;
    Ok(())
}

/// Find an existing foreign import for `origin` on one connection: rows
/// with the same source tool whose `origin_json.imported_from` matches the
/// origin by `foreign_session_id` (when supplied) or by `path`.
///
/// PARITY: `_find_foreign_import_on_conn` (upstream lines 127–139).
fn find_foreign_import_on_conn(
    conn: &Connection,
    origin: &Value,
) -> Result<Option<String>, rusqlite::Error> {
    let tool = origin.get("tool").and_then(Value::as_str).unwrap_or("");
    let mut stmt = conn.prepare(
        "SELECT id, origin_json FROM sessions \
         WHERE source = ? AND origin_json IS NOT NULL",
    )?;
    let rows = stmt.query_map(rusqlite::params![tool], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let foreign_id = origin
        .get("foreign_session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    for row in rows {
        let (id, origin_json) = row?;
        let Some(origin_json) = origin_json else {
            continue;
        };
        let parsed = hermes_utils::safe_json_loads(&origin_json, serde_json::json!({}));
        let Some(imported_from) = parsed.get("imported_from").and_then(Value::as_object) else {
            continue;
        };
        let imported_tool = imported_from
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or("");
        if imported_tool != tool {
            continue;
        }
        if !foreign_id.is_empty() {
            let imported_foreign = imported_from
                .get("foreign_session_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if imported_foreign == foreign_id {
                return Ok(Some(id));
            }
        } else {
            let imported_path = imported_from
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            let origin_path = origin.get("path").and_then(Value::as_str).unwrap_or("");
            if imported_path == origin_path {
                return Ok(Some(id));
            }
        }
    }
    Ok(None)
}

impl SessionDB {
    /// Look up an existing foreign import for `origin`, or None.
    ///
    /// PARITY: `find_foreign_import` (upstream lines 141–143).
    pub fn find_foreign_import(&self, origin: &Value) -> Result<Option<String>, WriteError> {
        let conn = self.writer_conn();
        find_foreign_import_on_conn(&conn, origin).map_err(WriteError::Sqlite)
    }

    /// Adopt or mint a foreign snapshot in one transaction, including its
    /// provenance. BEGIN IMMEDIATE serializes duplicate clicks across
    /// connections/processes; an already-adopted origin returns the
    /// EXISTING session id with `already_imported: true`.
    ///
    /// PARITY: `import_foreign_history` (upstream lines 145–173): mints a
    /// 12-hex session id, validates through the shared portability
    /// validator, suffixes a colliding title with `({id[-12:]})`, stamps
    /// `origin_json` + `profile_name`.
    pub fn import_foreign_history(
        &self,
        origin: &Value,
        messages: &[Value],
        title: &str,
        cwd: Option<&str>,
        profile: &str,
    ) -> Result<Value, WriteError> {
        let session_id = new_session_id(12);
        let payload = json!({
            "id": session_id,
            "source": origin.get("tool").cloned().unwrap_or(Value::Null),
            "title": title,
            "cwd": cwd.map(|c| json!(c)).unwrap_or(Value::Null),
            "messages": messages,
        });
        let mut totals = ImportTotals::default();
        let (sid, clean_session, clean_messages) = validate_import_entry(
            &payload,
            &std::collections::HashSet::new(),
            &mut totals.messages,
            &mut totals.bytes,
        )
        .map_err(|(_sid, message)| WriteError::ValueError(message))?;

        let f = |conn: &Connection| -> Result<Value, WriteError> {
            if let Some(existing) = find_foreign_import_on_conn(conn, origin)? {
                return Ok(json!({"session_id": existing, "already_imported": true}));
            }
            // Titles are globally unique within a profile: preserve a
            // readable title while giving same-text conversations room.
            let title_dup: Option<i64> = conn
                .query_row(
                    "SELECT 1 FROM sessions WHERE title = ?",
                    rusqlite::params![title],
                    |r| r.get(0),
                )
                .optional()?;
            let mut clean = clean_session.clone();
            if title_dup.is_some() {
                let suffix = &sid[sid.len().saturating_sub(12)..];
                clean
                    .as_object_mut()
                    .expect("object")
                    .insert("title".to_string(), json!(format!("{title} ({suffix})")));
            }
            import_session_row(conn, &clean, &clean_messages, &sid)?;
            let origin_json = serde_json::to_string(&json!({"imported_from": origin}))
                .map_err(|e| WriteError::ValueError(e.to_string()))?;
            conn.execute(
                "UPDATE sessions SET origin_json = ?, profile_name = ? WHERE id = ?",
                rusqlite::params![origin_json, profile, sid],
            )?;
            Ok(json!({"session_id": sid, "already_imported": false}))
        };
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }

    /// Adopt *session_id*'s full compression lineage from *donor_db*
    /// (stranded-bot-session heal, #93091/#93296): pure composition
    /// `donor.export_session_lineage()` → `self.import_sessions()`; runtime
    /// fields reset, already-present ids skipped (idempotent). With
    /// `retire_donor` and a complete adoption, donor rows are ARCHIVED
    /// (never deleted) with `end_reason='adopted_by_profile'` — deliberately
    /// NOT in the recoverable set, so resurrection cannot undo an adoption.
    ///
    /// PARITY: `adopt_session_lineage_from` (upstream lines 323–370) +
    /// `_retire_donor_segment` (372–396). PORT SEAMS: the export-patch
    /// TOCTOU oracle case is skipped (needs a donor-method monkeypatch);
    /// retirement failures catch per-segment instead of upstream's
    /// blanket try/except.
    pub fn adopt_session_lineage_from(
        &self,
        donor: &SessionDB,
        session_id: &str,
        retire_donor: bool,
    ) -> Result<Value, WriteError> {
        let payload = donor.export_session_lineage(session_id)?;
        let Some(payload) = payload else {
            return Ok(json!({
                "ok": false, "adopted": false, "donor_retired": false,
                "error": format!("session {session_id:?} not found in donor store"),
            }));
        };
        let segments: Vec<Value> = payload
            .get("segments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(|| vec![payload.clone()]);

        // Divergence guard: a segment we will SKIP (already here) may have
        // kept growing in the donor after a partial adoption; retiring it
        // would strand those messages behind a non-recoverable archive.
        // Still import, but refuse to retire.
        let mut donor_ahead = false;
        for seg in &segments {
            let Some(seg_id) = seg.get("id").and_then(Value::as_str) else {
                continue;
            };
            if self.get_session(seg_id)?.is_none() {
                continue;
            }
            let donor_count = seg
                .get("messages")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            let local_count = self.get_messages_dicts(seg_id, false, None, 0)?.len();
            if donor_count > local_count {
                donor_ahead = true;
                log::warn!(
                    "adoption divergence: donor segment {seg_id} has {donor_count} messages, \
local copy has {local_count} — donor will NOT be retired"
                );
            }
        }

        let result = self.import_sessions(&segments)?;
        let adopted_total = result.imported + result.skipped;
        let adopted = result.ok && adopted_total == segments.len();
        if !adopted {
            log::warn!(
                "adoption of {session_id} did not complete: imported={} skipped={} of {} segment(s); errors={:?}",
                result.imported, result.skipped, segments.len(), result.errors
            );
        }

        let mut donor_retired = false;
        if adopted && retire_donor && !donor_ahead {
            donor_retired = segments
                .iter()
                .filter_map(|seg| seg.get("id").and_then(Value::as_str))
                .all(|seg_id| self.retire_donor_segment(donor, seg_id));
        }
        let mut out = import_result_value(&result);
        out.as_object_mut()
            .expect("object")
            .insert("adopted".to_string(), json!(adopted));
        out.as_object_mut()
            .expect("object")
            .insert("donor_retired".to_string(), json!(donor_retired));
        Ok(out)
    }

    /// Archive one adopted donor segment; False when skipped or failed.
    /// TOCTOU close-out: the divergence guard used EXPORT-TIME counts;
    /// re-read both stores right before stamping so donor growth never
    /// lands behind a non-recoverable archive.
    ///
    /// PARITY: `_retire_donor_segment` (upstream lines 372–396).
    fn retire_donor_segment(&self, donor: &SessionDB, seg_id: &str) -> bool {
        (|| -> Result<bool, WriteError> {
            let donor_now = donor.get_messages_dicts(seg_id, false, None, 0)?.len();
            let local_now = self.get_messages_dicts(seg_id, false, None, 0)?.len();
            if donor_now > local_now {
                log::warn!(
                    "adoption divergence at retire time: donor segment {seg_id} grew to \
{donor_now} messages (local {local_now}) — leaving donor unretired"
                );
                return Ok(false);
            }
            // First end_reason wins in end_session(); reopen so the adoption
            // boundary is stamped even on ended segments.
            donor.reopen_session(seg_id)?;
            donor.end_session(seg_id, "adopted_by_profile")?;
            donor.set_session_archived(seg_id, true)?;
            Ok(true)
        })()
        .unwrap_or_else(|e| {
            log::warn!("failed to retire donor segment {seg_id} after adoption: {e}");
            false
        })
    }
}

fn import_error(index: usize, session_id: &str, error: &str) -> Value {
    if session_id.is_empty() {
        json!({"index": index, "error": error})
    } else {
        json!({"index": index, "session_id": session_id, "error": error})
    }
}

fn import_text_or_none(value: Value, field: &str) -> Result<Option<String>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s)),
        _ => Err(format!("{} must be a string", field)),
    }
}

fn import_json_object_or_none(value: Value, field: &str) -> Result<Option<String>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) => match serde_json::from_str::<Value>(&s) {
            Ok(Value::Object(_)) => Ok(Some(s)),
            Ok(_) => Err(format!("{} must be a JSON object", field)),
            Err(_) => Err(format!("{} must be valid JSON", field)),
        },
        Value::Object(_) => serde_json::to_string(&value)
            .map(Some)
            .map_err(|_| format!("{} must be JSON serializable", field)),
        _ => Err(format!("{} must be a JSON object", field)),
    }
}

fn float_or_none(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| v.as_f64())
}

fn import_int_or_none(value: Value, field: &str) -> Result<Option<i64>, String> {
    // Python `int(value)` coercion for JSON values (numbers, "12" strings).
    match value {
        Value::Null => Ok(None),
        Value::Number(n) => n
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("{} must be an integer", field)),
        Value::String(s2) => s2
            .parse::<i64>()
            .map(Some)
            .map_err(|_| format!("{} must be an integer", field)),
        _ => Err(format!("{} must be an integer", field)),
    }
}

fn int_or_default(value: Option<&Value>, default: i64) -> i64 {
    value
        .and_then(|v| match v {
            Value::Number(n) => n.as_i64(),
            _ => None,
        })
        .unwrap_or(default)
}

fn truthy_value(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64() != Some(0),
        _ => true,
    }
}

fn reasoning_json_value(v: Value) -> Value {
    // `_reasoning_json_value`: parse JSON strings, keep everything else.
    if let Value::String(s) = &v {
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            return parsed;
        }
    }
    v
}

/// Build the shared appendable row type from an imported message dict.
fn message_input_from_import(m: &Value) -> MessageInput {
    let obj = m.as_object().unwrap();
    MessageInput {
        role: value_text(obj.get("role")).unwrap_or_default(),
        content: obj.get("content").cloned(),
        tool_name: value_text(obj.get("tool_name")),
        tool_calls: obj.get("tool_calls").cloned(),
        tool_call_id: value_text(obj.get("tool_call_id")),
        token_count: obj.get("token_count").and_then(|v| v.as_i64()),
        finish_reason: value_text(obj.get("finish_reason")),
        reasoning: value_text(obj.get("reasoning")),
        reasoning_content: value_text(obj.get("reasoning_content")),
        reasoning_details: obj.get("reasoning_details").cloned(),
        codex_reasoning_items: obj.get("codex_reasoning_items").cloned(),
        codex_message_items: obj.get("codex_message_items").cloned(),
        platform_message_id: value_text(obj.get("platform_message_id")),
        message_id: value_text(obj.get("message_id")),
        observed: truthy_value(obj.get("observed")),
        effect_disposition: value_text(obj.get("effect_disposition")),
        timestamp: obj.get("timestamp").and_then(|v| v.as_f64()),
        api_content: value_text(obj.get("api_content")),
        display_kind: value_text(obj.get("display_kind")),
        display_metadata: obj.get("display_metadata").cloned(),
    }
}

impl MessageInput {
    /// Accessor for the naming-free reasoning sidecar keys (import sanitizer).
    fn reasoning_field_mut(&mut self, key: &str) -> Option<&mut Option<Value>> {
        match key {
            "reasoning_details" => Some(&mut self.reasoning_details),
            "codex_reasoning_items" => Some(&mut self.codex_reasoning_items),
            "codex_message_items" => Some(&mut self.codex_message_items),
            _ => None,
        }
    }
}

/// `_cwd_prefix_clause`: exact cwd match plus LIKE prefixes for subpaths.
pub(crate) fn cwd_prefix_clause(cwd_prefix: &str) -> (String, Vec<String>) {
    let prefix = cwd_prefix.trim_end_matches(['/', '\\']).to_string();
    let prefix = if prefix.is_empty() {
        cwd_prefix.to_string()
    } else {
        prefix
    };
    let esc = common::escape_like(&prefix);
    (
        "(s.cwd = ? OR s.cwd LIKE ? ESCAPE '\\' OR s.cwd LIKE ? ESCAPE '\\')".to_string(),
        vec![prefix, format!("{}/%", esc), format!("{}\\%%", esc)],
    )
}

/// `_workspace_key_clause`: git_repo_root equals key, or cwd under it for
/// rows that predate per-session git metadata.
fn workspace_key_clause(key: &str) -> (String, Vec<String>) {
    let prefix = key.trim_end_matches(['/', '\\']).to_string();
    let prefix = if prefix.is_empty() {
        key.to_string()
    } else {
        prefix
    };
    let (cwd_clause, cwd_params) = cwd_prefix_clause(&prefix);
    let mut params = vec![prefix.clone()];
    params.extend(cwd_params);
    (
        format!(
            "(s.git_repo_root = ? OR (COALESCE(s.git_repo_root, '') = '' AND {}))",
            cwd_clause
        ),
        params,
    )
}
