//! Full-text / trigram / CJK message search and FTS maintenance.
//!
//! PARITY: hermes_state_search.py @ b9aa928 (2,305 LOC). Deferred with the
//! "surface read helpers" unit (PLAN §5/§6): `search_sessions_by_id`
//! depends on `list_sessions_rich`, which is NOT yet ported — it is omitted
//! here and lands with that unit.

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::common;
use crate::compression_prefix;

use super::state::{SessionDB, WriteError};

pub const FTS_MERGE_MAX_PAGES_PER_INDEX: i64 = 500;
const FTS_MERGE_COMMANDS_PER_PASS: usize = 4;
const FTS_REBUILD_CHUNK_ROWS: i64 = 500;
const FTS_REBUILD_DUTY_FACTOR: f64 = 4.0;
const FTS_REBUILD_MIN_PAUSE: f64 = 0.2;
const FTS_TABLES: [&str; 3] = ["messages_fts", "messages_fts_trigram", "messages_fts_cjk"];

const SEARCH_MESSAGE_RESULT_FIELDS: [&str; 10] = [
    "id",
    "session_id",
    "role",
    "snippet",
    "timestamp",
    "tool_name",
    "source",
    "model",
    "session_started",
    "context",
];

/// `(_MALFORMED_SCHEMA_MARKERS)` hermes_state.py @ b9aa928 (1243).
const MALFORMED_SCHEMA_MARKERS: [&str; 2] = [
    "malformed database schema",
    "database disk image is malformed",
];

static SPECIAL_CHARS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[+{}():"^]"#).unwrap());
static STAR_COLLAPSE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*+").unwrap());
static LEADING_STAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(^|\s)\*").unwrap());
static LEADING_OPERATOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(AND|OR|NOT)\b\s*").unwrap());
static TRAILING_OPERATOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\s+(AND|OR|NOT)\s*$").unwrap());
static DOTTED_TERM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\w+(?:[._-]\w+)+)\b").unwrap());
static QUOTED_NON_SPACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""[^"]*"|\S+"#).unwrap());
// ── helpers ────────────────────────────────────────────────────────────────

fn malformed_or_fts_corrupt(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    MALFORMED_SCHEMA_MARKERS
        .iter()
        .any(|marker| m.contains(marker))
        || (m.contains("fts5") && m.contains("corrupt"))
}

fn log_warn(msg: &str) {
    eprintln!("[hermes-state] WARN: {}", msg);
}

fn rusqlite_msg(e: &rusqlite::Error) -> String {
    match e {
        rusqlite::Error::SqliteFailure(_, Some(m)) => m.to_string(),
        rusqlite::Error::SqliteFailure(_, None) => String::new(),
        other => other.to_string(),
    }
}

/// `_is_fts_write_corruption_error` @ b9aa928 (3241).
fn is_fts_write_corruption_error(e: &rusqlite::Error) -> bool {
    malformed_or_fts_corrupt(&rusqlite_msg(e))
}

pub fn is_cjk_codepoint(cp: u32) -> bool {
    (0x4E00..=0x9FFF).contains(&cp)      // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&cp)  // CJK Extension A
        || (0x20000..=0x2A6DF).contains(&cp) // CJK Extension B
        || (0x3000..=0x303F).contains(&cp) // CJK Symbols
        || (0x3040..=0x309F).contains(&cp) // Hiragana
        || (0x30A0..=0x30FF).contains(&cp) // Katakana
        || (0xAC00..=0xD7AF).contains(&cp) // Hangul Syllables
}

pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| is_cjk_codepoint(c as u32))
}

pub fn count_cjk(text: &str) -> usize {
    text.chars().filter(|c| is_cjk_codepoint(*c as u32)).count()
}

pub fn has_lone_cjk_run(text: &str) -> bool {
    let mut run = 0usize;
    for ch in text.chars() {
        if is_cjk_codepoint(ch as u32) {
            run += 1;
        } else {
            if run == 1 {
                return true;
            }
            run = 0;
        }
    }
    run == 1
}

pub fn trigram_eligible_tokens(query: &str) -> bool {
    let tokens: Vec<&str> = query
        .trim_matches('"')
        .split_whitespace()
        .filter(|t| !["AND", "OR", "NOT"].contains(&t.to_ascii_uppercase().as_str()))
        .collect();
    !tokens.is_empty() && tokens.iter().all(|t| t.chars().count() >= 3)
}

/// Strip surrogates is a no-op for Rust strings (kept as a parity seam).
#[allow(dead_code)]
fn _scrub_surrogates(s: Option<String>) -> Option<String> {
    s
}

/// `_content_text_for_contains` + handoff-prefix detection vendored from
/// agent/context_compressor.py (classify_summary_content subset). The full
/// compressor lands with the agent crate (P2); until then `list_recent_user
/// _messages` skips rows using the vendored prefixes.
fn is_context_summary_content(content: Option<&Value>) -> bool {
    let Some(v) = content else {
        return false;
    };
    let text = match v {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                match item {
                    Value::String(s) => parts.push(s.clone()),
                    Value::Object(o) => {
                        if let Some(Value::String(t)) = o.get("text") {
                            parts.push(t.clone());
                        }
                    }
                    _ => {}
                }
            }
            parts.join("\n")
        }
        Value::Number(n) => n.to_string(),
        _ => return false,
    };
    let text = text.trim_start().to_string();
    if let Some(idx) = text.find(compression_prefix::MERGED_SUMMARY_DELIMITER) {
        let after = text[idx + compression_prefix::MERGED_SUMMARY_DELIMITER.len()..]
            .trim_start()
            .to_string();
        return starts_with_summary_prefix(&after);
    }
    starts_with_summary_prefix(&text)
}

fn starts_with_summary_prefix(text: &str) -> bool {
    compression_prefix::HISTORICAL_SUMMARY_PREFIXES
        .iter()
        .chain(std::iter::once(&compression_prefix::SUMMARY_PREFIX))
        .chain(std::iter::once(&compression_prefix::LEGACY_SUMMARY_PREFIX))
        .any(|prefix| text.starts_with(prefix))
}

// ── projection validation ──────────────────────────────────────────────────

fn search_message_fields(fields: Option<&[String]>) -> Result<Option<Vec<String>>, String> {
    let Some(fields) = fields else {
        return Ok(None);
    };
    let requested: std::collections::HashSet<&str> =
        fields.iter().map(|s| s.as_str()).collect();
    let unknown: Vec<&str> = requested
        .iter()
        .filter(|f| !SEARCH_MESSAGE_RESULT_FIELDS.contains(f))
        .copied()
        .collect();
    if !unknown.is_empty() {
        let mut sorted = unknown.clone();
        sorted.sort_unstable();
        return Err(format!(
            "unknown search result field(s): {}",
            sorted.join(", ")
        ));
    }
    Ok(Some(
        SEARCH_MESSAGE_RESULT_FIELDS
            .iter()
            .filter(|f| requested.contains(**f))
            .map(|s| s.to_string())
            .collect(),
    ))
}

impl SessionDB {
    fn _fts_table_exists(&self, name: &str) -> bool {
        let conn = self.writer_conn();
        let sql = format!("SELECT 1 FROM {} LIMIT 0", name);
        conn.execute(&sql, []).is_ok()
    }

    fn _fts_index_known_empty(&self, conn: &Connection) -> bool {
        match conn.query_row("SELECT COUNT(*) FROM messages_fts_docsize", [], |r| {
            r.get::<_, i64>(0)
        }) {
            Ok(n) => n == 0,
            Err(_) => true, // absent table counts as empty
        }
    }

    fn _reset_fts_index_to_empty(&self, conn: &Connection) {
        for tbl in ["messages_fts", "messages_fts_trigram"] {
            // FTS5 'delete-all' — O(1) truncate for external-content tables.
            let sql = format!("INSERT INTO {}({}) VALUES('delete-all')", tbl, tbl);
            let _ = conn.execute(&sql, []);
        }
    }

    fn _seed_fts_rebuild_markers(
        &self,
        conn: &Connection,
        force: bool,
    ) -> Result<i64, WriteError> {
        let existing_hw: Option<i64> = conn
            .query_row(
                "SELECT value FROM state_meta WHERE key = 'fts_rebuild_high_water'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(hw) = existing_hw {
            if !force {
                let progress: Option<i64> = conn
                    .query_row(
                        "SELECT value FROM state_meta WHERE key = 'fts_rebuild_progress'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()?;
                if progress.is_none() {
                    // high_water without progress: fts_rebuild_step treats
                    // missing progress as "done by another process" and
                    // optimize would no-op then stamp. Re-seed progress so
                    // the chunk loop runs.
                    if !self._fts_index_known_empty(conn) {
                        self._reset_fts_index_to_empty(conn);
                    }
                    conn.execute(
                        "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_progress', '0') \
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        [],
                    )?;
                }
                return Ok(hw);
            }
        }
        let hw: i64 = conn
            .query_row("SELECT COALESCE(MAX(id), 0) FROM messages", [], |r| r.get(0))?;
        conn.execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_high_water', ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![hw.to_string()],
        )?;
        conn.execute(
            "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_progress', '0') \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
        Ok(hw)
    }

    fn _has_fts_trash(&self, conn: &Connection) -> bool {
        schema_has_fts_trash(conn)
    }

    fn _repair_optimize_bookkeeping(&self) -> Result<(), WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let existing_hw: Option<i64> = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = 'fts_rebuild_high_water'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(_hw) = existing_hw {
                let progress: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM state_meta WHERE key = 'fts_rebuild_progress'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()?;
                if progress.is_none() {
                    if !self._fts_index_known_empty(conn) {
                        self._reset_fts_index_to_empty(conn);
                    }
                    conn.execute(
                        "INSERT INTO state_meta (key, value) VALUES ('fts_rebuild_progress', '0') \
                         ON CONFLICT(key) DO UPDATE SET value = '0'",
                        [],
                    )?;
                }
                return Ok(());
            }
            // No markers. On a still-legacy DB demote owns marker creation.
            if crate::schema::db_has_legacy_inline_fts(conn)? {
                return Ok(());
            }
            // Non-legacy empty external index (demote crash window / premature
            // stamp): seed a full backfill claim.
            if crate::schema::fts_external_index_empty_with_messages(conn) {
                conn.execute(
                    "DELETE FROM state_meta WHERE key = 'fts_storage_version'",
                    [],
                )?;
                self._seed_fts_rebuild_markers(conn, true)?;
            }
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// True while `optimize_fts_storage()` has work to do.
    /// PARITY: hermes_state_search.py fts_optimize_available @ b9aa928
    pub fn fts_optimize_available(&self) -> bool {
        if !self.fts_enabled() || self.read_only {
            return false;
        }
        let conn = self.writer_conn();
        if crate::schema::db_has_legacy_inline_fts(&conn).unwrap_or(false) {
            return true;
        }
        let has_pending: bool = conn
            .query_row(
                "SELECT 1 FROM state_meta WHERE key = 'fts_rebuild_high_water' LIMIT 1",
                [],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();
        if has_pending {
            return true;
        }
        if self.fts_cjk_loaded() {
            let has_cjk_pending: bool = conn
                .query_row(
                    "SELECT 1 FROM state_meta WHERE key IN ('fts_cjk_rebuild_high_water', ?) LIMIT 1",
                    rusqlite::params![crate::common::FTS_CJK_STALE_KEY],
                    |r| r.get::<_, i64>(0),
                )
                .optional()
                .ok()
                .flatten()
                .is_some();
            if has_cjk_pending {
                return true;
            }
        }
        if self._has_fts_trash(&conn) {
            return true;
        }
        crate::schema::fts_external_index_empty_with_messages(&conn)
    }

    fn _fts_teardown_trash_step(&self) -> bool {
        let trash: Vec<String> = {
            let conn = self.writer_conn();
            let mut stmt = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE ? ESCAPE '\\'",
                )
                .unwrap();
            stmt.query_map(
                rusqlite::params![format!("{}%", crate::state::SessionDB::FTS_TRASH_PREFIX.replace('_', "\\_"))],
                |r| r.get::<_, String>(0),
            )
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
        };
        if trash.is_empty() {
            return false;
        }
        let tbl = trash[0].clone();

        let f = |conn: &Connection| -> Result<bool, WriteError> {
            // PRAGMA table_info: (cid, name, type, notnull, dflt, pk)
            let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", tbl))?;
            let pk_info: Vec<(String, String, i64)> = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(5)?,
                    ))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default();
            let pk_cols: Vec<String> = pk_info
                .iter()
                .filter(|(_, _, pk)| *pk > 0)
                .map(|(name, _, _)| name.clone())
                .collect();
            let key = if pk_cols.is_empty() {
                "rowid".to_string()
            } else {
                pk_cols.join(", ")
            };

            if pk_cols.len() == 1 && (pk_info.is_empty() || pk_info[0].1 == "INTEGER") {
                let marker_key = format!("fts_teardown_{}_progress", tbl);
                let high_water: i64 = match conn.query_row(
                    "SELECT value FROM state_meta WHERE key = ?",
                    rusqlite::params![marker_key],
                    |r| r.get::<_, String>(0),
                ) {
                    Ok(v) => v.parse().unwrap_or(0),
                    Err(_) => 0,
                };
                let upper_rows: Vec<i64> = {
                    let mut stmt2 = conn.prepare(&format!(
                        "SELECT {} FROM {} WHERE {} > ? \
                         ORDER BY {} LIMIT {}",
                        key, tbl, key, key, FTS_REBUILD_CHUNK_ROWS
                    ))?;
                    let x = stmt2
                        .query_map(rusqlite::params![high_water], |r| r.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    x
                };
                if upper_rows.is_empty() {
                    conn.execute(&format!("DROP TABLE IF EXISTS {}", tbl), [])?;
                    conn.execute("DELETE FROM state_meta WHERE key = ?", rusqlite::params![marker_key])?;
                    log_warn(&format!("Old FTS shadow table {} torn down.", tbl));
                    return Ok(true);
                }
                let upper = *upper_rows.last().unwrap();
                let cur = conn.execute(
                    &format!("DELETE FROM {} WHERE {} > ? AND {} <= ?", tbl, key, key),
                    rusqlite::params![high_water, upper],
                )?;
                if cur > 0 {
                    conn.execute(
                        "INSERT INTO state_meta (key, value) VALUES (?, ?) \
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        rusqlite::params![marker_key, upper.to_string()],
                    )?;
                }
                return Ok(true);
            }

            // Compound-key or rowid trash table: legacy chunked delete.
            let cur = conn.execute(
                &format!(
                    "DELETE FROM {} WHERE ({}) IN (SELECT {} FROM {} LIMIT {})",
                    tbl, key, key, tbl, FTS_REBUILD_CHUNK_ROWS
                ),
                [],
            )?;
            if cur == 0 {
                conn.execute(&format!("DROP TABLE IF EXISTS {}", tbl), [])?;
                log_warn(&format!("Old FTS shadow table {} torn down.", tbl));
            }
            Ok(true)
        };
        self.execute_write(&f, None).unwrap_or(true) // transient — retry
    }

    /// Backfill one chunk of the deferred FTS rebuild. True while work remains.
    /// PARITY: hermes_state_search.py fts_rebuild_step @ b9aa928
    pub fn fts_rebuild_step(&self) -> bool {
        if !self.fts_enabled() {
            return false;
        }
        let Some(high_water_raw) = self.get_meta("fts_rebuild_high_water") else {
            return false;
        };
        let Ok(high_water) = high_water_raw.parse::<i64>() else {
            return false;
        };
        let include_trigram = self.trigram_available();

        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let Some(progress_raw): Option<String> = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = 'fts_rebuild_progress'",
                    [],
                    |r| r.get(0),
                )
                .optional()?
            else {
                return Ok(false); // finished by another process
            };
            let Ok(progress) = progress_raw.parse::<i64>() else {
                return Ok(false);
            };
            if progress >= high_water {
                return Ok(false);
            }
            let upper = (progress + FTS_REBUILD_CHUNK_ROWS).min(high_water);
            conn.execute(
                "INSERT INTO messages_fts(rowid, content, tool_name, tool_calls) \
                 SELECT id, content, tool_name, tool_calls FROM messages \
                 WHERE id > ? AND id <= ?",
                rusqlite::params![progress, upper],
            )?;
            if include_trigram {
                conn.execute(
                    "INSERT INTO messages_fts_trigram(rowid, content, tool_name, tool_calls) \
                     SELECT id, content, tool_name, tool_calls FROM messages \
                     WHERE id > ? AND id <= ? AND role <> 'tool'",
                    rusqlite::params![progress, upper],
                )?;
            }
            conn.execute(
                "UPDATE state_meta SET value = ? WHERE key = 'fts_rebuild_progress'",
                rusqlite::params![upper.to_string()],
            )?;
            Ok(upper < high_water)
        };
        let more = match self.execute_write(&f, None) {
            Ok(v) => v,
            Err(_) => return true, // transient — caller retries
        };
        if !more {
            if let Some(status) = self.fts_rebuild_status() {
                if status["indexed"].as_i64().unwrap_or(0) >= status["total"].as_i64().unwrap_or(0) {
                    self._fts_rebuild_finish();
                }
            }
            false
        } else {
            true
        }
    }

    /// Return deferred-rebuild progress, or None when no rebuild pending.
    /// PARITY: hermes_state_search.py fts_rebuild_status @ b9aa928
    pub fn fts_rebuild_status(&self) -> Option<Value> {
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare("SELECT key, value FROM state_meta WHERE key IN (?, ?)")
            .ok()?;
        let rows: Vec<(String, String)> = stmt
            .query_map(
                rusqlite::params!["fts_rebuild_high_water", "fts_rebuild_progress"],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()?
            .collect::<Result<_, _>>()
            .ok()?;
        let high_water = rows
            .iter()
            .find(|(k, _)| k == "fts_rebuild_high_water")
            .map(|(_, v)| v.clone());
        let total = high_water?.parse::<i64>().unwrap_or(0);
        if total <= 0 {
            return None;
        }
        let progress = rows
            .iter()
            .find(|(k, _)| k == "fts_rebuild_progress")
            .map(|(_, v)| v.parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        let pct = (100 * progress / total).min(100);
        Some(json!({
            "pending": true,
            "total": total,
            "indexed": progress,
            "percent": pct,
        }))
    }

    fn _fts_rebuild_finish(&self) {
        let include_trigram = self.trigram_available();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let hw_row: Option<String> = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = 'fts_rebuild_high_water'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(hw_raw) = hw_row {
                let Ok(hw) = hw_raw.parse::<i64>() else {
                    return Err(WriteError::Runtime("high_water unparseable".into()));
                };
                let (lo, hi) = (hw - 1000, hw + 1000);
                conn.execute(
                    "INSERT INTO messages_fts(rowid, content, tool_name, tool_calls) \
                     SELECT m.id, m.content, m.tool_name, m.tool_calls \
                     FROM messages m \
                     WHERE m.id > ? AND m.id <= ? \
                     AND NOT EXISTS (SELECT 1 FROM messages_fts_docsize d WHERE d.id = m.id)",
                    rusqlite::params![lo, hi],
                )?;
                if include_trigram {
                    conn.execute(
                        "INSERT INTO messages_fts_trigram(rowid, content, tool_name, tool_calls) \
                         SELECT m.id, m.content, m.tool_name, m.tool_calls \
                         FROM messages m \
                         WHERE m.id > ? AND m.id <= ? AND m.role <> 'tool' \
                         AND NOT EXISTS (SELECT 1 FROM messages_fts_trigram_docsize d WHERE d.id = m.id)",
                        rusqlite::params![lo, hi],
                    )?;
                }
            }
            conn.execute(
                "DELETE FROM state_meta WHERE key IN ('fts_rebuild_high_water', 'fts_rebuild_progress')",
                [],
            )?;
            Ok(())
        };
        let _ = self.execute_write(&f, None);
        log_warn("Deferred FTS rebuild complete — all messages indexed.");
    }

    fn _fts_cjk_rebuild_finish(&self) {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let hw_row: Option<String> = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = 'fts_cjk_rebuild_high_water'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(hw_raw) = hw_row {
                let Ok(hw) = hw_raw.parse::<i64>() else {
                    return Err(WriteError::Runtime("cjk high_water unparseable".into()));
                };
                let (lo, hi) = (hw - 1000, hw + 1000);
                conn.execute(
                    "INSERT INTO messages_fts_cjk(rowid, content, tool_name, tool_calls) \
                     SELECT m.id, m.content, m.tool_name, m.tool_calls \
                     FROM messages m \
                     WHERE m.id > ? AND m.id <= ? AND m.role <> 'tool' \
                     AND NOT EXISTS (SELECT 1 FROM messages_fts_cjk_docsize d WHERE d.id = m.id)",
                    rusqlite::params![lo, hi],
                )?;
            }
            conn.execute(
                "DELETE FROM state_meta WHERE key IN ('fts_cjk_rebuild_high_water', 'fts_cjk_rebuild_progress')",
                [],
            )?;
            Ok(())
        };
        let _ = self.execute_write(&f, None);
        self.set_fts_cjk_available(true);
        log_warn("CJK FTS index backfill complete — serving CJK search.");
    }

    /// Like fts_rebuild_status but for the CJK-bigram index.
    /// PARITY: hermes_state_search.py fts_cjk_rebuild_status @ b9aa928
    pub fn fts_cjk_rebuild_status(&self) -> Option<Value> {
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare("SELECT key, value FROM state_meta WHERE key IN (?, ?)")
            .ok()?;
        let rows: Vec<(String, String)> = stmt
            .query_map(
                rusqlite::params!["fts_cjk_rebuild_high_water", "fts_cjk_rebuild_progress"],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()?
            .collect::<Result<_, _>>()
            .ok()?;
        let high_water = rows
            .iter()
            .find(|(k, _)| k == "fts_cjk_rebuild_high_water")
            .map(|(_, v)| v.clone());
        let total = high_water?.parse::<i64>().unwrap_or(0);
        if total <= 0 {
            return None;
        }
        let progress = rows
            .iter()
            .find(|(k, _)| k == "fts_cjk_rebuild_progress")
            .map(|(_, v)| v.parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        let pct = (100 * progress / total).min(100);
        Some(json!({
            "pending": true,
            "total": total,
            "indexed": progress,
            "percent": pct,
        }))
    }

    /// Backfill one chunk of the CJK index. True while work remains.
    /// PARITY: hermes_state_search.py fts_cjk_rebuild_step @ b9aa928
    pub fn fts_cjk_rebuild_step(&self) -> bool {
        if !self.fts_enabled() || !self.fts_cjk_loaded() {
            return false;
        }
        let Some(high_water_raw) = self.get_meta("fts_cjk_rebuild_high_water") else {
            return false;
        };
        let Ok(high_water) = high_water_raw.parse::<i64>() else {
            return false;
        };
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let Some(progress_raw): Option<String> = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = 'fts_cjk_rebuild_progress'",
                    [],
                    |r| r.get(0),
                )
                .optional()?
            else {
                return Ok(false);
            };
            let Ok(progress) = progress_raw.parse::<i64>() else {
                return Ok(false);
            };
            if progress >= high_water {
                return Ok(false);
            }
            let upper = (progress + FTS_REBUILD_CHUNK_ROWS).min(high_water);
            conn.execute(
                "INSERT INTO messages_fts_cjk(rowid, content, tool_name, tool_calls) \
                 SELECT id, content, tool_name, tool_calls FROM messages \
                 WHERE id > ? AND id <= ? AND role <> 'tool'",
                rusqlite::params![progress, upper],
            )?;
            conn.execute(
                "UPDATE state_meta SET value = ? WHERE key = 'fts_cjk_rebuild_progress'",
                rusqlite::params![upper.to_string()],
            )?;
            Ok(upper < high_water)
        };
        let more = match self.execute_write(&f, None) {
            Ok(v) => v,
            Err(_) => return true,
        };
        if !more {
            if let Some(status) = self.fts_cjk_rebuild_status() {
                if status["indexed"].as_i64().unwrap_or(0) >= status["total"].as_i64().unwrap_or(0) {
                    self._fts_cjk_rebuild_finish();
                }
            }
            false
        } else {
            true
        }
    }

    /// Rebuild path for a stale cjk index (triggers were dropped).
    /// PARITY: hermes_state_search.py _fts_cjk_reset_if_stale @ b9aa928
    fn _fts_cjk_reset_if_stale(&self) {
        if !self.fts_cjk_loaded() {
            return;
        }
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let stale: Option<i64> = conn
                .query_row(
                    "SELECT 1 FROM state_meta WHERE key = ?",
                    rusqlite::params![crate::common::FTS_CJK_STALE_KEY],
                    |r| r.get(0),
                )
                .optional()?;
            if stale.is_none() {
                return Ok(false);
            }
            for trig in crate::common::_FTS_CJK_TRIGGERS {
                conn.execute(&format!("DROP TRIGGER IF EXISTS {}", trig), [])?;
            }
            conn.execute("DROP TABLE IF EXISTS messages_fts_cjk", [])?;
            conn.execute("DROP VIEW IF EXISTS messages_fts_cjk_src", [])?;
            conn.execute(
                "DELETE FROM state_meta WHERE key IN (?, 'fts_cjk_rebuild_high_water', 'fts_cjk_rebuild_progress')",
                rusqlite::params![crate::common::FTS_CJK_STALE_KEY],
            )?;
            Ok(true)
        };
        let was_stale = self.execute_write(&f, None).unwrap_or(false);
        if was_stale {
            // Recreate outside the write transaction — ensure_fts_cjk_schema
            // uses executescript-style multi-statement DDL (upstream rule:
            // never inside BEGIN IMMEDIATE).
            let conn = self.writer_conn();
            self.ensure_fts_cjk_schema(&conn);
        }
    }

    /// One-shot rebuild_fts after a corrupt-index write failure.
    /// PARITY: hermes_state.py SessionDB._try_runtime_fts_rebuild @ b9aa928
    fn _try_runtime_fts_rebuild(&self, exc: &rusqlite::Error) -> bool {
        if self.fts_runtime_rebuild_attempted() || !self.fts_enabled() {
            return false;
        }
        if !is_fts_write_corruption_error(exc) {
            return false;
        }
        self.set_fts_runtime_rebuild_attempted(true);
        log_warn("state.db read failed with an FTS-corruption error — attempting one-shot in-place FTS rebuild; canonical message rows are preserved.");
        let rebuilt = self.rebuild_fts();
        if rebuilt == 0 {
            log_warn("In-place FTS rebuild made no progress; the database needs the full offline repair path.");
            return false;
        }
        log_warn("state.db FTS indexes rebuilt in place; retrying the failed operation.");
        true
    }
}

fn schema_has_fts_trash(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name LIKE ? ESCAPE '\\' LIMIT 1",
        rusqlite::params![format!(
            "{}%",
            crate::state::SessionDB::FTS_TRASH_PREFIX.replace('_', "\\_")
        )],
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

// ── sanitizer ───────────────────────────────────────────────────────────────

/// Sanitize user input for safe use in FTS5 MATCH queries.
/// PARITY: hermes_state_search.py _sanitize_fts5_query @ b9aa928
pub fn sanitize_fts5_query(query: &str) -> String {
    let bound: String = query.chars().take(crate::common::MAX_FTS5_QUERY_CHARS).collect();

    // Step 1: extract balanced double-quoted phrases into placeholders.
    let mut quoted_parts: Vec<String> = Vec::new();
    let mut pieces: Vec<char> = Vec::new();
    let chars: Vec<char> = bound.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch != '"' {
            pieces.push(ch);
            i += 1;
            continue;
        }
        // find the closing quote
        let mut end = None;
        for (j, c) in chars.iter().enumerate().skip(i + 1) {
            if *c == '"' {
                end = Some(j);
                break;
            }
        }
        match end {
            None => {
                pieces.push(' ');
                i += 1;
            }
            Some(e) => {
                let quoted: String = chars[i..=e].iter().collect();
                quoted_parts.push(quoted);
                let marker = format!("\u{0}Q{}\u{0}", quoted_parts.len() - 1);
                pieces.extend(marker.chars());
                i = e + 1;
            }
        }
    }
    let mut sanitized: String = pieces.iter().collect();

    // Step 2: strip unmatched FTS5-special characters (`:` is the column
    // filter operator; unquoted `TODO: fix` parses as col:term → "no such
    // column", swallowed to zero results upstream).
    sanitized = SPECIAL_CHARS_RE.replace_all(&sanitized, " ").into_owned();

    // Step 3: collapse repeated * (e.g. "***") and remove leading *.
    sanitized = STAR_COLLAPSE_RE.replace_all(&sanitized, "*").into_owned();
    sanitized = LEADING_STAR_RE.replace_all(&sanitized, "$1").into_owned();

    // Step 4: remove dangling boolean operators at start/end.
    sanitized = LEADING_OPERATOR_RE.replace_all(sanitized.trim(), "").into_owned();
    sanitized = TRAILING_OPERATOR_RE.replace_all(sanitized.trim(), "").into_owned();

    // Step 5: wrap unquoted dotted/hyphenated terms in double quotes in a
    // single pass (avoids the sequential double-quoting bug).
    sanitized = DOTTED_TERM_RE
        .replace_all(&sanitized, |caps: &regex::Captures| format!("\"{}\"", &caps[1]))
        .into_owned();

    // Step 6: restore preserved quoted phrases.
    for (i, quoted) in quoted_parts.iter().enumerate() {
        let marker = format!("\u{0}Q{}\u{0}", i);
        sanitized = sanitized.replace(&marker, quoted);
    }
    sanitized.trim().to_string()
}

/// Best-effort name of the routing path a query takes (log-only).
/// PARITY: hermes_state_search.py _describe_search_path @ b9aa928
// Routing-path label for the slow-search log (log-only; best-effort).
fn describe_search_path(query: &str) -> String {
    let sanitized = sanitize_fts5_query(query);
    if sanitized.is_empty() {
        return "empty".to_string();
    }
    if !contains_cjk(&sanitized) {
        return "fts5".to_string();
    }
    "cjk_like_or_trigram".to_string()
}

fn context_text_preview(content: Option<&Value>) -> String {
    match content {
        Some(Value::Array(items)) => {
            let mut parts = Vec::new();
            for item in items {
                if let Value::Object(o) = item {
                    if let Some(Value::String(t)) = o.get("text") {
                        if !t.is_empty() {
                            parts.push(t.clone());
                        }
                    }
                }
            }
            let text = parts.join(" ");
            if text.trim().is_empty() {
                "[multimodal content]".to_string()
            } else {
                text
            }
        }
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

// ── anchored view + recent user messages ───────────────────────────────────

impl SessionDB {
    /// Anchored window plus session bookends for an FTS hit.
    /// PARITY: hermes_state_search.py get_anchored_view @ b9aa928
    pub fn get_anchored_view(
        &self,
        session_id: &str,
        around_message_id: i64,
        window: i64,
        bookend: i64,
        keep_roles: Option<&[String]>,
    ) -> Result<Value, WriteError> {
        let bookend = bookend.max(0);
        let primitive = self.get_messages_around(session_id, around_message_id, window)?;
        let window_rows = primitive["window"].as_array().cloned().unwrap_or_default();
        if window_rows.is_empty() {
            return Ok(json!({
                "window": [],
                "messages_before": 0,
                "messages_after": 0,
                "bookend_start": [],
                "bookend_end": [],
            }));
        }
        // Apply role filter, but never drop the anchor itself.
        let filtered_window = match keep_roles {
            Some(roles) => window_rows
                .iter()
                .filter(|m| {
                    m.get("id").and_then(|v| v.as_i64()) == Some(around_message_id)
                        || m.get("role")
                            .and_then(|v| v.as_str())
                            .is_some_and(|r| roles.iter().any(|x| x == r))
                })
                .cloned()
                .collect(),
            None => window_rows.clone(),
        };
        let window_min_id = window_rows[0]["id"].as_i64().unwrap_or(0);
        let window_max_id = window_rows[window_rows.len() - 1]["id"].as_i64().unwrap_or(0);

        let mut bookend_start_rows: Vec<Value> = Vec::new();
        let mut bookend_end_rows: Vec<Value> = Vec::new();
        if bookend > 0 {
            let conn = self.writer_conn();
            let role_clause = match keep_roles {
                Some(roles) if !roles.is_empty() => {
                    let ph = vec!["?"; roles.len()].join(",");
                    format!(" AND role IN ({})", ph)
                }
                _ => String::new(),
            };
            let mut params_start: Vec<Box<dyn rusqlite::ToSql>> = vec![
                Box::new(session_id.to_string()),
                Box::new(window_min_id),
            ];
            if let Some(roles) = keep_roles {
                for r in roles {
                    params_start.push(Box::new(r.clone()));
                }
            }
            params_start.push(Box::new(bookend));
            let sql_start = format!(
                "SELECT * FROM messages \
                 WHERE session_id = ? AND id < ?{} AND length(content) > 0 \
                 ORDER BY id ASC LIMIT ?",
                role_clause
            );
            let mut stmt_start = conn.prepare(&sql_start).map_err(WriteError::Sqlite)?;
            bookend_start_rows = stmt_start
                .query_map(
                    rusqlite::params_from_iter(params_start.iter().map(|p| p as &dyn rusqlite::ToSql)),
                    super::portability::message_row_to_value,
                )
                .map_err(WriteError::Sqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(WriteError::Sqlite)?;

            let mut params_end: Vec<Box<dyn rusqlite::ToSql>> = vec![
                Box::new(session_id.to_string()),
                Box::new(window_max_id),
            ];
            if let Some(roles) = keep_roles {
                for r in roles {
                    params_end.push(Box::new(r.clone()));
                }
            }
            params_end.push(Box::new(bookend));
            let sql_end = format!(
                "SELECT * FROM messages \
                 WHERE session_id = ? AND id > ?{} AND length(content) > 0 \
                 ORDER BY id DESC LIMIT ?",
                role_clause
            );
            let mut stmt_end = conn.prepare(&sql_end).map_err(WriteError::Sqlite)?;
            bookend_end_rows = stmt_end
                .query_map(
                    rusqlite::params_from_iter(params_end.iter().map(|p| p as &dyn rusqlite::ToSql)),
                    super::portability::message_row_to_value,
                )
                .map_err(WriteError::Sqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(WriteError::Sqlite)?;
            // End rows came back DESC for the LIMIT cap; flip to ASC.
            bookend_end_rows.reverse();
        }

        Ok(json!({
            "window": filtered_window,
            "messages_before": primitive["messages_before"],
            "messages_after": primitive["messages_after"],
            "bookend_start": bookend_start_rows,
            "bookend_end": bookend_end_rows,
        }))
    }

    /// The *limit* most-recent user messages, newest first, with previews.
    /// PARITY: hermes_state_search.py list_recent_user_messages @ b9aa928
    pub fn list_recent_user_messages(
        &self,
        session_id: &str,
        limit: i64,
        include_inactive: bool,
    ) -> Result<Vec<Value>, WriteError> {
        let active_clause = if include_inactive { "" } else { " AND active = 1" };
        let display_clause = " AND (display_kind IS NULL OR display_kind = '')";
        let fetch_limit = limit * 2 + 5;
        let sql = format!(
            "SELECT id, timestamp, content FROM messages \
             WHERE session_id = ? AND role = 'user'{} {} \
             ORDER BY id DESC LIMIT ?",
            active_clause, display_clause
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows: Vec<(i64, f64, Option<rusqlite::types::Value>)> = stmt
            .query_map(
                rusqlite::params![session_id, fetch_limit],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<_, _>>()
            .map_err(WriteError::Sqlite)?;

        let mut result: Vec<Value> = Vec::new();
        for (id, ts, raw_content) in rows {
            if (result.len() as i64) >= limit {
                break;
            }
            let decoded = super::crud::decode_content(raw_content);
            if is_context_summary_content(decoded.as_ref()) {
                // Compaction handoff — never a user-originated turn (#80622).
                continue;
            }
            let preview = match &decoded {
                Some(Value::Array(items)) => {
                    let mut parts = Vec::new();
                    for item in items {
                        if let Value::Object(o) = item {
                            if let Some(Value::String(t)) = o.get("text") {
                                if !t.is_empty() {
                                    parts.push(t.clone());
                                }
                            }
                        }
                    }
                    let preview = parts.join(" ");
                    if preview.trim().is_empty() {
                        "[multimodal content]".to_string()
                    } else {
                        preview
                    }
                }
                Some(Value::String(s)) => {
                    // A /skill turn embeds the whole skill body; show what the
                    // user typed instead of the skill's opening prose.
                    crate::skill::describe_skill_invocation(s, " — ")
                        .filter(|d| !d.is_empty())
                        .unwrap_or_else(|| s.clone())
                }
                _ => String::new(),
            };
            let collapsed: String = preview.split_whitespace().collect::<Vec<_>>().join(" ");
            let preview = if collapsed.chars().count() > 80 {
                let mut s: String = collapsed.chars().take(77).collect();
                s.push_str("...");
                s
            } else {
                collapsed
            };
            result.push(json!({"id": id, "timestamp": ts, "preview": preview}));
        }
        Ok(result)
    }
}

// ── search_messages ────────────────────────────────────────────────────────

impl SessionDB {
    // Same keyword envelope as upstream _run_trigram_search.
    #[allow(clippy::too_many_arguments)]
    fn _run_trigram_search(
        &self,
        raw_query: &str,
        table: &str,
        order_by_sql: &str,
        include_inactive: bool,
        source_filter: Option<&[String]>,
        exclude_sources: Option<&[String]>,
        role_filter: Option<&[String]>,
        limit: i64,
        offset: i64,
    ) -> Option<Vec<Value>> {
        let tokens: Vec<&str> = raw_query.split_whitespace().collect();
        let mut parts = Vec::new();
        for tok in tokens {
            let upper = tok.to_ascii_uppercase();
            if upper == "AND" || upper == "OR" || upper == "NOT" {
                parts.push(tok.to_string());
            } else {
                parts.push(format!("\"{}\"", tok.replace('"', "\"\"")));
            }
        }
        let trigram_query = parts.join(" ");
        let mut tri_where = vec![format!("{} MATCH ?", table)];
        let mut tri_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(trigram_query)];
        if !include_inactive {
            tri_where.push("(m.active = 1 OR m.compacted = 1)".to_string());
        }
        if let Some(filters) = source_filter {
            let ph = vec!["?"; filters.len()].join(",");
            tri_where.push(format!("s.source IN ({})", ph));
            for f in filters {
                tri_params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = exclude_sources {
            let ph = vec!["?"; filters.len()].join(",");
            tri_where.push(format!("s.source NOT IN ({})", ph));
            for f in filters {
                tri_params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = role_filter {
            if !filters.is_empty() {
                let ph = vec!["?"; filters.len()].join(",");
                tri_where.push(format!("m.role IN ({})", ph));
                for f in filters {
                    tri_params.push(Box::new(f.clone()));
                }
            }
        }
        let sql = format!(
            "SELECT m.id, m.session_id, m.role, \
                snippet({}, -1, '>>>', '<<<', '...', 40) AS snippet, \
                m.content, m.timestamp, m.tool_name, \
                s.source, s.model, s.started_at AS session_started \
             FROM {} \
             JOIN messages m ON m.id = {}.rowid \
             JOIN sessions s ON s.id = m.session_id \
             WHERE {} {} LIMIT ? OFFSET ?",
            table, table, table, tri_where.join(" AND "), order_by_sql,
        );
        tri_params.push(Box::new(limit));
        tri_params.push(Box::new(offset));
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).ok()?;
        let rows: Result<Vec<Value>, _> = stmt
            .query_map(
                rusqlite::params_from_iter(tri_params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                super::portability::row_to_value,
            )
            .ok()?
            .collect();
        rows.ok()
    }

    // Upstream's search_messages takes the same keyword envelope; keep the
    // flat positional mirror.
    #[allow(clippy::too_many_arguments)]
    pub fn search_messages(
        &self,
        query: &str,
        source_filter: Option<&[String]>,
        exclude_sources: Option<&[String]>,
        role_filter: Option<&[String]>,
        limit: i64,
        offset: i64,
        sort: Option<&str>,
        include_inactive: bool,
        fields: Option<&[String]>,
    ) -> Result<Vec<Value>, WriteError> {
        let started = std::time::Instant::now();
        let result_fields = search_message_fields(fields)
            .map_err(WriteError::ValueError)?;
        if !self.fts_enabled() {
            return Ok(vec![]);
        }
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        let sanitized = sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        // Normalise sort; anything not in the allowed set falls back to
        // rank-only.
        let sort_norm = match sort {
            Some(s) if s.trim().eq_ignore_ascii_case("newest") => Some("newest"),
            Some(s) if s.trim().eq_ignore_ascii_case("oldest") => Some("oldest"),
            _ => None,
        };
        let order_by_sql = match sort_norm {
            Some("newest") => "ORDER BY m.timestamp DESC, rank".to_string(),
            Some("oldest") => "ORDER BY m.timestamp ASC, rank".to_string(),
            _ => "ORDER BY rank".to_string(),
        };

        // Build WHERE clauses.
        let mut where_clauses = vec!["messages_fts MATCH ?".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(sanitized.clone())];
        if !include_inactive {
            where_clauses.push("(m.active = 1 OR m.compacted = 1)".to_string());
        }
        if let Some(filters) = source_filter {
            let ph = vec!["?"; filters.len()].join(",");
            where_clauses.push(format!("s.source IN ({})", ph));
            for f in filters {
                params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = exclude_sources {
            let ph = vec!["?"; filters.len()].join(",");
            where_clauses.push(format!("s.source NOT IN ({})", ph));
            for f in filters {
                params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = role_filter {
            if !filters.is_empty() {
                let ph = vec!["?"; filters.len()].join(",");
                where_clauses.push(format!("m.role IN ({})", ph));
                for f in filters {
                    params.push(Box::new(f.clone()));
                }
            }
        }
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let sql = format!(
            "SELECT m.id, m.session_id, m.role, \
                snippet(messages_fts, -1, '>>>', '<<<', '...', 40) AS snippet, \
                m.content, m.timestamp, m.tool_name, \
                s.source, s.model, s.started_at AS session_started \
             FROM messages_fts \
             JOIN messages m ON m.id = messages_fts.rowid \
             JOIN sessions s ON s.id = m.session_id \
             WHERE {} {} LIMIT ? OFFSET ?",
            where_clauses.join(" AND "),
            order_by_sql,
        );

        let conn = self.writer_conn();
        let mut matches: Vec<Value> = Vec::new();
        let is_cjk = contains_cjk(&sanitized);
        if is_cjk {
            matches = self._search_cjk(
                &sanitized,
                order_by_sql.as_str(),
                include_inactive,
                source_filter,
                exclude_sources,
                role_filter,
                limit,
                offset,
            );
        } else {
            let run = || -> Result<Vec<Value>, rusqlite::Error> {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(
                    rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                    super::portability::row_to_value,
                )?;
                rows.collect()
            };
            match run() {
                Ok(rows) => matches = rows,
                Err(e) => {
                    let msg = rusqlite_msg(&e);
                    if is_fts_write_corruption_error(&e) {
                        if self._try_runtime_fts_rebuild(&e) {
                            if let Ok(rows) = run() {
                                matches = rows;
                            }
                        } else {
                            // Upstream re-raises DatabaseError here; Rust has
                            // no exceptions, so log and return the empty set
                            // (documented fail-safe divergence).
                            log_warn(&format!(
                                "search_messages: FTS-corruption recovery refused/absent ({}); returning []",
                                msg
                            ));
                        }
                    }
                    // OperationalError (syntax) — upstream returns [].
                }
            }
        }

        if !is_cjk {
            // Deferred-rebuild supplement (schema v23): top results up with a
            // bounded LIKE scan over the unindexed id gap.
            if self.fts_rebuild_status().is_some() && (matches.len() as i64) < limit {
                let gap = self._search_unindexed_gap(
                        &sanitized,
                        limit.saturating_sub(matches.len() as i64),
                        include_inactive,
                        source_filter,
                        exclude_sources,
                        role_filter,
                    );
                    if let Ok(gap_matches) = gap {
                        let seen: std::collections::HashSet<i64> = matches
                            .iter()
                            .filter_map(|m| m.get("id").and_then(|v| v.as_i64()))
                            .collect();
                        for m in gap_matches {
                            if let Some(id) = m.get("id").and_then(|v| v.as_i64()) {
                                if !seen.contains(&id) {
                                    matches.push(m);
                                }
                            }
                        }
                    }
            }

            // Pure-Latin miss → retry on substring-capable indexes
            // (CJK-bigram then trigram) for the #54242 class.
            if matches.is_empty()
                && !(role_filter.is_some_and(|r| r.iter().any(|v| v == "tool")))
            {
                let fb_query = sanitized.trim_matches('"').trim().to_string();
                if self.fts_cjk_available() {
                    if let Some(fb) = self._run_trigram_search(
                        &fb_query,
                        "messages_fts_cjk",
                        &order_by_sql,
                        include_inactive,
                        source_filter,
                        exclude_sources,
                        role_filter,
                        limit,
                        offset,
                    ) {
                        matches = fb;
                    }
                }
                if matches.is_empty() && self.trigram_available() && trigram_eligible_tokens(&sanitized) {
                    if let Some(fb) = self._run_trigram_search(
                        &fb_query,
                        "messages_fts_trigram",
                        &order_by_sql,
                        include_inactive,
                        source_filter,
                        exclude_sources,
                        role_filter,
                        limit,
                        offset,
                    ) {
                        matches = fb;
                    }
                }
            }
        }

        // Add surrounding context (1 message before + after each match) only
        // when the selected projection consumes it.
        let wants_context = result_fields.as_ref().is_none_or(|f| f.iter().any(|x| x == "context"));
        if wants_context {
            for m in &mut matches {
                let mid = m.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                m["context"] = self._context_around(mid).unwrap_or_else(|_| json!([]));
            }
        }

        // Remove full content from result (snippet is enough, saves tokens).
        for m in &mut matches {
            if let Some(obj) = m.as_object_mut() {
                obj.remove("content");
            }
        }

        // Projection.
        if let Some(fields_list) = &result_fields {
            matches = matches
                .into_iter()
                .map(|m| {
                    let mut out = serde_json::Map::new();
                    for f in fields_list {
                        if let Some(v) = m.get(f) {
                            out.insert(f.clone(), v.clone());
                        }
                    }
                    Value::Object(out)
                })
                .collect();
        }

        // Slow-search instrumentation: upstream logs one line per slow search
        // with the routing path taken (HERMES_SEARCH_SLOW_MS, default 1000;
        // 0 logs every call). eprintln seam until hermes-logging is wired.
        let threshold: f64 = std::env::var("HERMES_SEARCH_SLOW_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000.0);
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if elapsed_ms >= threshold {
            let q: String = query.chars().take(200).collect();
            log_warn(&format!(
                "slow session search: path={} elapsed={:.0}ms rows={} query={:?}",
                describe_search_path(query),
                elapsed_ms,
                matches.len(),
                q,
            ));
        }
        Ok(matches)
    }

    fn _context_around(&self, message_id: i64) -> Result<Value, WriteError> {
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare(
                "WITH target AS (SELECT session_id, timestamp, id FROM messages WHERE id = ?) \
                 SELECT role, content \
                 FROM (SELECT m.id, m.timestamp, m.role, m.content \
                       FROM messages m JOIN target t ON t.session_id = m.session_id \
                       WHERE (m.timestamp < t.timestamp) \
                          OR (m.timestamp = t.timestamp AND m.id < t.id) \
                       ORDER BY m.timestamp DESC, m.id DESC LIMIT 1) \
                 UNION ALL \
                 SELECT role, content FROM messages WHERE id = ? \
                 UNION ALL \
                 SELECT role, content FROM (SELECT m.id, m.timestamp, m.role, m.content \
                       FROM messages m JOIN target t ON t.session_id = m.session_id \
                       WHERE (m.timestamp > t.timestamp) \
                          OR (m.timestamp = t.timestamp AND m.id > t.id) \
                       ORDER BY m.timestamp ASC, m.id ASC LIMIT 1)",
            )
            .map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![message_id, message_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<rusqlite::types::Value>>(1)?,
                ))
            })
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        let mut context_msgs = Vec::new();
        for (role, raw) in rows {
            let decoded = super::crud::decode_content(raw);
            let preview: String = context_text_preview(decoded.as_ref());
            let preview: String = preview.chars().take(200).collect();
            context_msgs.push(json!({"role": role, "content": preview}));
        }
        Ok(Value::Array(context_msgs))
    }

    // Mirrors the upstream CJK routing call shape.
    #[allow(clippy::too_many_arguments)]
    fn _search_cjk(
        &self,
        query: &str,
        order_by_sql: &str,
        include_inactive: bool,
        source_filter: Option<&[String]>,
        exclude_sources: Option<&[String]>,
        role_filter: Option<&[String]>,
        limit: i64,
        offset: i64,
    ) -> Vec<Value> {
        let raw_query = query.trim_matches('"').trim().to_string();
        let cjk_count = count_cjk(&raw_query);
        let tokens_for_check: Vec<&str> = raw_query
            .split_whitespace()
            .filter(|t| !["AND", "OR", "NOT"].contains(&t.to_ascii_uppercase().as_str()))
            .filter(|t| contains_cjk(t))
            .collect();
        let any_short_cjk = tokens_for_check.iter().any(|t| count_cjk(t) < 3);
        let wants_tool_rows = role_filter.is_some_and(|r| r.iter().any(|v| v == "tool"));
        let mut succeeded = false;
        let mut matches: Vec<Value> = Vec::new();

        // ── CJK-bigram route (messages_fts_cjk) — serves EVERY CJK query
        // shape when available, except tool-role queries and lone 1-char
        // CJK runs.
        if self.fts_cjk_available() && !wants_tool_rows && !has_lone_cjk_run(&raw_query) {
            let tokens: Vec<&str> = raw_query.split_whitespace().collect();
            let mut parts = Vec::new();
            for tok in tokens {
                let upper = tok.to_ascii_uppercase();
                if upper == "AND" || upper == "OR" || upper == "NOT" {
                    parts.push(tok.to_string());
                } else {
                    parts.push(format!("\"{}\"", tok.replace('"', "\"\"")));
                }
            }
            let cjk_query = parts.join(" ");
            let mut cjk_where = vec!["messages_fts_cjk MATCH ?".to_string()];
            let mut cjk_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(cjk_query)];
            if !include_inactive {
                cjk_where.push("(m.active = 1 OR m.compacted = 1)".to_string());
            }
            if let Some(filters) = source_filter {
                let ph = vec!["?"; filters.len()].join(",");
                cjk_where.push(format!("s.source IN ({})", ph));
                for f in filters {
                    cjk_params.push(Box::new(f.clone()));
                }
            }
            if let Some(filters) = exclude_sources {
                let ph = vec!["?"; filters.len()].join(",");
                cjk_where.push(format!("s.source NOT IN ({})", ph));
                for f in filters {
                    cjk_params.push(Box::new(f.clone()));
                }
            }
            if let Some(filters) = role_filter {
                if !filters.is_empty() {
                    let ph = vec!["?"; filters.len()].join(",");
                    cjk_where.push(format!("m.role IN ({})", ph));
                    for f in filters {
                        cjk_params.push(Box::new(f.clone()));
                    }
                }
            }
            let sql = format!(
                "SELECT m.id, m.session_id, m.role, \
                    snippet(messages_fts_cjk, -1, '>>>', '<<<', '...', 40) AS snippet, \
                    m.content, m.timestamp, m.tool_name, \
                    s.source, s.model, s.started_at AS session_started \
                 FROM messages_fts_cjk \
                 JOIN messages m ON m.id = messages_fts_cjk.rowid \
                 JOIN sessions s ON s.id = m.session_id \
                 WHERE {} {} LIMIT ? OFFSET ?",
                cjk_where.join(" AND "),
                order_by_sql,
            );
            cjk_params.push(Box::new(limit));
            cjk_params.push(Box::new(offset));
            let conn = self.writer_conn();
            let run = || -> Result<Vec<Value>, rusqlite::Error> {
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(
                    rusqlite::params_from_iter(cjk_params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                    super::portability::row_to_value,
                )?;
                rows.collect()
            };
            match run() {
                Ok(rows) => {
                    matches = rows;
                    succeeded = true;
                }
                Err(e) => {
                    if is_fts_write_corruption_error(&e) && self._try_runtime_fts_rebuild(&e) {
                        if let Ok(rows) = run() {
                            matches = rows;
                            succeeded = true;
                        }
                    }
                    // missing tokenizer / syntax → fall through
                }
            }
        }

        if !succeeded
            && cjk_count >= 3
            && !any_short_cjk
            && self.trigram_available()
            && !wants_tool_rows
        {
            if let Some(fb) = self._run_trigram_search(
                &raw_query,
                "messages_fts_trigram",
                order_by_sql,
                include_inactive,
                source_filter,
                exclude_sources,
                role_filter,
                limit,
                offset,
            ) {
                matches = fb;
                succeeded = true;
            }
        }

        if !succeeded {
            // Short / mixed CJK query, trigram unavailable, or trigram <3
            // CJK chars: LIKE substring fallback (multi-token OR per term).
            let non_op_tokens: Vec<String> = raw_query
                .split_whitespace()
                .filter(|t| !["AND", "OR", "NOT"].contains(&t.to_ascii_uppercase().as_str()))
                .map(str::to_string)
                .collect();
            let non_op_tokens: Vec<String> =
                if non_op_tokens.is_empty() { vec![raw_query.clone()] } else { non_op_tokens };
            let mut token_clauses = Vec::new();
            let mut like_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for tok in &non_op_tokens {
                let esc = common::escape_like(tok);
                token_clauses.push(
                    "(m.content LIKE ? ESCAPE '\\' OR m.tool_name LIKE ? ESCAPE '\\' OR m.tool_calls LIKE ? ESCAPE '\\')"
                        .to_string(),
                );
                let needle = format!("%{}%", esc);
                for _ in 0..3 {
                    like_params.push(Box::new(needle.clone()));
                }
            }
            let mut like_where = vec![format!("({})", token_clauses.join(" OR "))];
            if !include_inactive {
                like_where.push("(m.active = 1 OR m.compacted = 1)".to_string());
            }
            if let Some(filters) = source_filter {
                let ph = vec!["?"; filters.len()].join(",");
                like_where.push(format!("s.source IN ({})", ph));
                for f in filters {
                    like_params.push(Box::new(f.clone()));
                }
            }
            if let Some(filters) = exclude_sources {
                let ph = vec!["?"; filters.len()].join(",");
                like_where.push(format!("s.source NOT IN ({})", ph));
                for f in filters {
                    like_params.push(Box::new(f.clone()));
                }
            }
            if let Some(filters) = role_filter {
                if !filters.is_empty() {
                    let ph = vec!["?"; filters.len()].join(",");
                    like_where.push(format!("m.role IN ({})", ph));
                    for f in filters {
                        like_params.push(Box::new(f.clone()));
                    }
                }
            }
            let sql = format!(
                "SELECT m.id, m.session_id, m.role, \
                    substr(m.content, max(1, instr(m.content, ?) - 40), 120) AS snippet, \
                    m.content, m.timestamp, m.tool_name, \
                    s.source, s.model, s.started_at AS session_started \
                 FROM messages m \
                 JOIN sessions s ON s.id = m.session_id \
                 WHERE {} ORDER BY m.timestamp DESC LIMIT ? OFFSET ?",
                like_where.join(" AND "),
            );
            // instr() snippet uses the first search token.
            like_params.insert(0, Box::new(non_op_tokens[0].clone()));
            like_params.push(Box::new(limit));
            like_params.push(Box::new(offset));
            let conn = self.writer_conn();
            let mut stmt = match conn.prepare(&sql) {
                Ok(s) => s,
                Err(_) => return matches,
            };
            let rows: Result<Vec<Value>, _> = stmt
                .query_map(
                    rusqlite::params_from_iter(like_params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                    super::portability::row_to_value,
                )
                .map_err(WriteError::Sqlite)
                .and_then(|it| it.collect::<Result<Vec<_>, _>>().map_err(WriteError::Sqlite));
            if let Ok(rows) = rows {
                matches = rows;
            }
        }
        matches
    }

    /// LIKE-scan the rows the deferred rebuild hasn't indexed yet.
    /// PARITY: hermes_state_search.py _search_unindexed_gap @ b9aa928
    fn _search_unindexed_gap(
        &self,
        fts_query: &str,
        limit: i64,
        include_inactive: bool,
        source_filter: Option<&[String]>,
        exclude_sources: Option<&[String]>,
        role_filter: Option<&[String]>,
    ) -> Result<Vec<Value>, WriteError> {
        if limit <= 0 {
            return Ok(vec![]);
        }
        let Some(status) = self.fts_rebuild_status() else {
            return Ok(vec![]);
        };
        let progress = status["indexed"].as_i64().unwrap_or(0);
        let high_water = status["total"].as_i64().unwrap_or(0);

        // Degrade the FTS query to LIKE terms.
        let mut terms: Vec<String> = Vec::new();
        for raw_tok in QUOTED_NON_SPACE_RE
            .find_iter(fts_query)
            .map(|m| m.as_str().trim_matches('"').trim_matches('*').trim().to_string())
        {
            let upper = raw_tok.to_ascii_uppercase();
            if raw_tok.is_empty() || ["AND", "OR", "NOT", "NEAR"].contains(&upper.as_str()) {
                continue;
            }
            terms.push(raw_tok);
        }
        if terms.is_empty() {
            return Ok(vec![]);
        }

        let mut where_clauses = vec!["m.id > ? AND m.id <= ?".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(progress), Box::new(high_water)];
        for term in &terms {
            let esc = common::escape_like(term);
            where_clauses.push(
                "(m.content LIKE ? ESCAPE '\\' OR m.tool_name LIKE ? ESCAPE '\\' OR m.tool_calls LIKE ? ESCAPE '\\')"
                    .to_string(),
            );
            let needle = format!("%{}%", esc);
            for _ in 0..3 {
                params.push(Box::new(needle.clone()));
            }
        }
        if !include_inactive {
            where_clauses.push("(m.active = 1 OR m.compacted = 1)".to_string());
        }
        if let Some(filters) = source_filter {
            let ph = vec!["?"; filters.len()].join(",");
            where_clauses.push(format!("s.source IN ({})", ph));
            for f in filters {
                params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = exclude_sources {
            let ph = vec!["?"; filters.len()].join(",");
            where_clauses.push(format!("s.source NOT IN ({})", ph));
            for f in filters {
                params.push(Box::new(f.clone()));
            }
        }
        if let Some(filters) = role_filter {
            if !filters.is_empty() {
                let ph = vec!["?"; filters.len()].join(",");
                where_clauses.push(format!("m.role IN ({})", ph));
                for f in filters {
                    params.push(Box::new(f.clone()));
                }
            }
        }
        let sql = format!(
            "SELECT m.id, m.session_id, m.role, \
                substr(m.content, max(1, instr(m.content, ?) - 40), 120) AS snippet, \
                m.content, m.timestamp, m.tool_name, \
                s.source, s.model, s.started_at AS session_started \
             FROM messages m \
             JOIN sessions s ON s.id = m.session_id \
             WHERE {} ORDER BY m.timestamp DESC LIMIT ?",
            where_clauses.join(" AND "),
        );
        params.insert(0, Box::new(terms[0].clone()));
        params.push(Box::new(limit));
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                super::portability::row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }

    /// Merge fragmented FTS5 b-tree segments into one per index.
    /// PARITY: hermes_state_search.py optimize_fts @ b9aa928
    pub fn optimize_fts(&self) -> i64 {
        let mut optimized = 0i64;
        let conn = self.writer_conn();
        for tbl in FTS_TABLES {
            if !self._fts_table_exists(tbl) {
                continue;
            }
            let sql = format!("INSERT INTO {}({}) VALUES('optimize')", tbl, tbl);
            match conn.execute(&sql, []) {
                Ok(_) => optimized += 1,
                Err(e) => log_warn(&format!("FTS optimize failed for {}: {}", tbl, rusqlite_msg(&e))),
            }
        }
        optimized
    }

    /// Rebuild FTS5 indexes from the canonical messages table.
    /// PARITY: hermes_state_search.py rebuild_fts @ b9aa928
    pub fn rebuild_fts(&self) -> i64 {
        let mut rebuilt = 0i64;
        let conn = self.writer_conn();
        for tbl in FTS_TABLES {
            if !self._fts_table_exists(tbl) {
                continue;
            }
            let sql = format!("INSERT INTO {}({}) VALUES('rebuild')", tbl, tbl);
            match conn.execute(&sql, []) {
                Ok(_) => {
                    let _ = conn.execute_batch("COMMIT");
                    rebuilt += 1;
                }
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    log_warn(&format!("FTS rebuild failed for {}: {}", tbl, rusqlite_msg(&e)));
                }
            }
        }
        rebuilt
    }

    /// Run bounded FTS5 'merge' commands against each present index.
    /// PARITY: hermes_state_search.py _merge_fts_incrementally @ b9aa928
    pub(crate) fn _merge_fts_incrementally(
        &self,
        max_pages: i64,
        max_commands: Option<usize>,
    ) -> Result<usize, WriteError> {
        if max_pages <= 0 {
            return Err(WriteError::ValueError("max_pages must be greater than zero".into()));
        }
        let max_commands = max_commands.unwrap_or(FTS_MERGE_COMMANDS_PER_PASS);
        if max_commands == 0 {
            return Err(WriteError::ValueError("max_commands must be greater than zero".into()));
        }
        let mut executed = 0usize;
        let conn = self.writer_conn();
        for tbl in FTS_TABLES {
            if !self._fts_table_exists(tbl) {
                continue;
            }
            if !self.fts_usermerge_floor_applied() {
                let sql = format!("INSERT INTO {}({}, rank) VALUES('usermerge', 2)", tbl, tbl);
                conn.execute(&sql, []).map_err(WriteError::Sqlite)?;
            }
            for _ in 0..max_commands {
                let before = conn.total_changes();
                let sql = format!("INSERT INTO {}({}, rank) VALUES('merge', ?)", tbl, tbl);
                conn.execute(&sql, rusqlite::params![max_pages]).map_err(WriteError::Sqlite)?;
                executed += 1;
                if conn.total_changes() - before < 2 {
                    break;
                }
            }
        }
        self.set_fts_usermerge_floor_applied(true);
        Ok(executed)
    }
}

// Reserved for the deferred `search_sessions_by_id` (depends on
// list_sessions_rich — see the "surface read helpers" unit, PLAN §5/§6).

// ── optimize_fts_storage ───────────────────────────────────────────────────

impl SessionDB {
    /// Demote the legacy inline FTS vtables and stage their shadow tables
    /// for chunked teardown. Returns MAX(messages.id) as the rebuild high
    /// water. Markers are written in the same BEGIN IMMEDIATE as the demote,
    /// before the empty v23 schema is created (schema creation uses
    /// executescript-style DDL and cannot run inside the transaction).
    /// PARITY: hermes_state_search.py _demote_legacy_fts_to_trash @ b9aa928
    fn _demote_legacy_fts_to_trash(&self) -> Result<i64, WriteError> {
        let f = |conn: &Connection| -> Result<i64, WriteError> {
            self.drop_fts_triggers(conn);
            conn.execute("DROP VIEW IF EXISTS messages_fts_trigram_src", [])?;
            let had: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' \
                     AND name IN ('messages_fts', 'messages_fts_trigram') \
                     AND sql LIKE 'CREATE VIRTUAL TABLE%' LIMIT 1",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if had {
                conn.execute_batch("PRAGMA writable_schema=ON")?;
                conn.execute(
                    "DELETE FROM sqlite_master WHERE type = 'table' \
                     AND name IN ('messages_fts', 'messages_fts_trigram') \
                     AND sql LIKE 'CREATE VIRTUAL TABLE%'",
                    [],
                )?;
                conn.execute_batch("PRAGMA writable_schema=RESET")?;
                let shadows: Vec<String> = {
                    let sql = "SELECT name FROM sqlite_master WHERE type = 'table' \
                               AND (name LIKE 'messages_fts_%' ESCAPE '\\' \
                               OR name LIKE 'messages_fts_trigram_%' ESCAPE '\\')";
                    let mut stmt = conn.prepare(sql)?;
                    let x = stmt
                        .query_map([], |r| r.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    x
                };
                for sh in shadows {
                    let renamed = format!("{}_x_{}", crate::state::SessionDB::FTS_TRASH_PREFIX, sh);
                    conn.execute(&format!("ALTER TABLE {} RENAME TO {}", sh, renamed), [])?;
                }
            }
            let hw = self._seed_fts_rebuild_markers(conn, true)?;
            conn.execute("DELETE FROM state_meta WHERE key = 'fts_optimize_available'", [])?;
            Ok(hw)
        };
        let hw = self.execute_write(&f, None)?;

        // Create the empty v23 schema outside the write transaction.
        {
            let conn = self.writer_conn();
            let base_ok = self.ensure_fts_schema(&conn, "messages_fts", crate::common::FTS_SQL);
            let trigram_ok =
                self.ensure_fts_schema(&conn, "messages_fts_trigram", crate::common::FTS_TRIGRAM_SQL);
            self.set_trigram_available(trigram_ok);
            if !base_ok {
                return Err(WriteError::Runtime(
                    "failed to create v23 messages_fts during optimize-storage demote".into(),
                ));
            }
            conn.execute_batch("COMMIT").ok();
        }
        Ok(hw)
    }

    /// Migrate a legacy v22 inline-FTS DB to the v23 external-content schema,
    /// foreground and to completion. Safe to re-run.
    /// PARITY: hermes_state_search.py optimize_fts_storage @ b9aa928
    #[allow(clippy::too_many_arguments)]
    pub fn optimize_fts_storage(
        &self,
        progress_cb: Option<&dyn Fn(&Value)>,
        vacuum: bool,
    ) -> Result<Value, WriteError> {
        if !self.fts_enabled() {
            return Ok(json!({"ok": false, "reason": "fts5_unavailable"}));
        }
        if self.read_only {
            return Ok(json!({"ok": false, "reason": "read_only"}));
        }
        self._repair_optimize_bookkeeping()?;

        let legacy = {
            let conn = self.writer_conn();
            crate::schema::db_has_legacy_inline_fts(&conn).unwrap_or(false)
        };
        let pending = self.get_meta("fts_rebuild_high_water").is_some();
        if legacy && !pending {
            self._demote_legacy_fts_to_trash()?;
        } else if pending && !legacy {
            let conn = self.writer_conn();
            let base_ok = self.ensure_fts_schema(&conn, "messages_fts", crate::common::FTS_SQL);
            let trigram_ok =
                self.ensure_fts_schema(&conn, "messages_fts_trigram", crate::common::FTS_TRIGRAM_SQL);
            self.set_trigram_available(trigram_ok);
            if !base_ok {
                return Err(WriteError::Runtime(
                    "failed to re-create v23 messages_fts on optimize-storage resume".into(),
                ));
            }
            conn.execute_batch("COMMIT").ok();
        }

        self._fts_cjk_reset_if_stale();
        if self.fts_cjk_loaded() {
            let conn = self.writer_conn();
            self.ensure_fts_cjk_schema(&conn);
            conn.execute_batch("COMMIT").ok();
        }

        let emit = |phase: &str, progress_cb: Option<&dyn Fn(&Value)>| {
            if let Some(cb) = progress_cb {
                let st = self.fts_rebuild_status().or_else(|| self.fts_cjk_rebuild_status());
                let payload = match &st {
                    Some(st) => json!({
                        "phase": phase,
                        "percent": st["percent"],
                        "indexed": st["indexed"],
                        "total": st["total"],
                    }),
                    None => json!({"phase": phase, "percent": 100, "indexed": 0, "total": 0}),
                };
                cb(&payload);
            }
        };

        let pause = |chunk_seconds: f64| {
            let amount = FTS_REBUILD_MIN_PAUSE.max(chunk_seconds * FTS_REBUILD_DUTY_FACTOR);
            std::thread::sleep(std::time::Duration::from_secs_f64(amount));
        };

        // Phase 1: backfill (standard index).
        emit("backfill", progress_cb);
        loop {
            let t0 = std::time::Instant::now();
            if !self.fts_rebuild_step() {
                break;
            }
            emit("backfill", progress_cb);
            pause(t0.elapsed().as_secs_f64());
        }
        emit("backfill", progress_cb);

        // Phase 1b: CJK backfill.
        loop {
            let t0 = std::time::Instant::now();
            if !self.fts_cjk_rebuild_step() {
                break;
            }
            emit("backfill", progress_cb);
            pause(t0.elapsed().as_secs_f64());
        }

        // Phase 2: tear down demoted legacy shadow tables.
        emit("teardown", progress_cb);
        loop {
            let t0 = std::time::Instant::now();
            if !self._fts_teardown_trash_step() {
                break;
            }
            emit("teardown", progress_cb);
            pause(t0.elapsed().as_secs_f64());
        }

        // Refuse to stamp "optimized" while work remains or the base index is
        // still empty against a non-empty messages table.
        let (still_pending, still_trash) = {
            let conn = self.writer_conn();
            let pending: bool = conn
                .query_row(
                    "SELECT 1 FROM state_meta WHERE key = 'fts_rebuild_high_water' LIMIT 1",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .optional()
                .ok()
                .flatten()
                .is_some();
            let trash = self._has_fts_trash(&conn);
            (pending, trash)
        };
        let empty_index = {
            let conn = self.writer_conn();
            crate::schema::fts_external_index_empty_with_messages(&conn)
        };
        if still_pending || still_trash || empty_index {
            let reason = if still_pending || empty_index {
                "backfill_incomplete"
            } else {
                "teardown_incomplete"
            };
            log_warn(&format!(
                "FTS storage optimization did not settle ({}): pending={} trash={} empty_index={}",
                reason, still_pending, still_trash, empty_index,
            ));
            return Ok(json!({"ok": false, "reason": reason, "vacuumed": Value::Null}));
        }

        // Phase 3: reclaim freed pages to the OS.
        let vacuum_ok = if vacuum {
            emit("vacuum", progress_cb);
            let conn = self.writer_conn();
            let v = match conn.execute_batch("VACUUM") {
                Ok(()) => Some(true),
                Err(e) => {
                    log_warn(&format!("VACUUM after FTS optimize failed: {}", rusqlite_msg(&e)));
                    Some(false)
                }
            };
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
            v
        } else {
            None
        };

        // Phase 4: stamp the FTS storage layout as current.
        let settle = |conn: &Connection| -> Result<Option<&'static str>, WriteError> {
            let pending: bool = conn
                .query_row(
                    "SELECT 1 FROM state_meta WHERE key = 'fts_rebuild_high_water' LIMIT 1",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if pending {
                return Ok(Some("backfill_incomplete"));
            }
            if self._has_fts_trash(conn) {
                return Ok(Some("teardown_incomplete"));
            }
            if crate::schema::fts_external_index_empty_with_messages(conn) {
                return Ok(Some("backfill_incomplete"));
            }
            conn.execute(
                "INSERT INTO state_meta (key, value) VALUES ('fts_storage_version', ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![crate::common::FTS_STORAGE_VERSION.to_string()],
            )?;
            conn.execute("DELETE FROM state_meta WHERE key = 'fts_optimize_available'", [])?;
            conn.execute(
                "UPDATE schema_version SET version = ? WHERE version < ?",
                rusqlite::params![crate::common::SCHEMA_VERSION, crate::common::SCHEMA_VERSION],
            )?;
            Ok(None)
        };
        let refusal = self.execute_write(&settle, None)?;
        if let Some(reason) = refusal {
            log_warn(&format!("FTS storage optimization settle refused ({})", reason));
            return Ok(json!({"ok": false, "reason": reason, "vacuumed": vacuum_ok}));
        }
        emit("done", progress_cb);
        log_warn(&format!(
            "FTS storage optimization complete (layout v{}).",
            crate::common::FTS_STORAGE_VERSION
        ));
        Ok(json!({"ok": true, "vacuumed": vacuum_ok}))
    }
}
