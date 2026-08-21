//! SessionSchemaMixin — schema creation, column reconciliation, FTS DDL.
//!
//! PARITY: hermes_state_schema.py @ b9aa928 (all methods of
//! SessionSchemaMixin, plus the schema-adjacent helpers that live on
//! SessionDB in hermes_state.py: _is_fts5_unavailable_error /
//! _is_trigram_unavailable_error / _db_has_legacy_inline_fts /
//! _warn_* / _ensure_fts_cjk_schema / _drop_fts_triggers /
//! _ensure_fts_schema / _has_fts_trash / _fts_external_index_empty_with_messages).
//!
//! Mixed into `SessionDB` as an inherent impl block (Rust lets a type's
//! impls span multiple files; the upstream Python class is literally a
//! multi-file mixin, so the translation is 1:1).

use rusqlite::{Connection, OptionalExtension};

use crate::common::*;
use crate::state::SessionDB;

// ---------------------------------------------------------------------------
// schema_read_probe_statements()
// ---------------------------------------------------------------------------

/// Cache for `schema_read_probe_statements()` — parsing SCHEMA_SQL spins up
/// an in-memory SQLite database, so derive the statements once per process.
// PARITY: hermes_state_schema.py _READ_PROBE_STATEMENTS @ b9aa928
static READ_PROBE_STATEMENTS: once_cell::sync::Lazy<Vec<String>> =
    once_cell::sync::Lazy::new(|| {
        let tables = parse_schema_columns(SCHEMA_SQL);
        let mut out = Vec::new();
        for (table, cols) in tables {
            let cols_sql: Vec<String> = cols
                .iter()
                .map(|(col, _)| {
                    format!(
                        "\"{}\".\"{}\"",
                        table.replace('"', "\"\""),
                        col.replace('"', "\"\"")
                    )
                })
                .collect();
            out.push(format!(
                "SELECT {} FROM \"{}\" LIMIT 0",
                cols_sql.join(", "),
                table.replace('"', "\"\"")
            ));
        }
        out
    });

/// SELECT statements that fail iff a live store is behind SCHEMA_SQL.
// PARITY: hermes_state_schema.py schema_read_probe_statements @ b9aa928
pub fn schema_read_probe_statements() -> &'static [String] {
    &READ_PROBE_STATEMENTS
}

// ---------------------------------------------------------------------------
// Static helpers
// ---------------------------------------------------------------------------

/// The FTS5-unavailable error family.
// PARITY: hermes_state.py _is_fts5_unavailable_error @ b9aa928
pub fn is_fts5_unavailable_error(msg: &str) -> bool {
    let err = msg.to_ascii_lowercase();
    if err.contains("no such module") && err.contains("fts5") {
        return true;
    }
    if err.contains("no such tokenizer: trigram") {
        return true;
    }
    if err.contains("no such tokenizer: cjk_unicode61") {
        return true;
    }
    false
}

/// True when only an optional tokenizer is missing (FTS5 itself works).
// PARITY: hermes_state.py _is_trigram_unavailable_error @ b9aa928
pub fn is_trigram_unavailable_error(msg: &str) -> bool {
    let err = msg.to_ascii_lowercase();
    err.contains("no such tokenizer: trigram") || err.contains("no such tokenizer: cjk_unicode61")
}

/// True when `messages_fts` exists in ANY pre-v23 shape.
// PARITY: hermes_state.py _db_has_legacy_inline_fts @ b9aa928
pub fn db_has_legacy_inline_fts(conn: &Connection) -> rusqlite::Result<bool> {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'messages_fts'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let Some(sql) = sql else {
        return Ok(false);
    };
    Ok(!sql.contains("tool_name"))
}

/// Parse `schema_sql` into `[(table, [(column, reconstructed_type)])]`.
///
/// Uses an in-memory SQLite database to parse the SQL — SQLite itself
/// handles all syntax, so there are zero regex edge cases.
// PARITY: hermes_state_schema.py _parse_schema_columns @ b9aa928
pub fn parse_schema_columns(schema_sql: &str) -> Vec<(String, Vec<(String, String)>)> {
    let ref_conn = Connection::open_in_memory().expect("in-memory sqlite");
    ref_conn
        .execute_batch(schema_sql)
        .expect("SCHEMA_SQL parses on a clean in-memory database");
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut stmt = ref_conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .expect("list tables");
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .expect("query tables")
        .collect::<Result<_, _>>()
        .expect("collect tables");
    for tbl in tables {
        let mut cols: Vec<(String, String)> = Vec::new();
        let mut pt = ref_conn
            .prepare(&format!("PRAGMA table_info(\"{}\")", tbl.replace('"', "\"\"")))
            .expect("table_info");
        let rows = pt
            .query_map([], |r| {
                let col_name: String = r.get(1)?;
                let col_type: String = r.get(2)?;
                let notnull: i64 = r.get(3)?;
                let default: Option<String> = r.get(4)?;
                let pk: i64 = r.get(5)?;
                Ok((col_name, col_type, notnull, default, pk))
            })
            .expect("query table_info");
        for row in rows {
            let (col_name, col_type, notnull, default, pk) = row.expect("row");
            let mut parts: Vec<String> = Vec::new();
            if !col_type.is_empty() {
                parts.push(col_type);
            }
            if notnull != 0 && pk == 0 {
                parts.push("NOT NULL".to_string());
            }
            if let Some(d) = default {
                parts.push(format!("DEFAULT {}", d));
            }
            cols.push((col_name, parts.join(" ")));
        }
        out.push((tbl, cols));
    }
    out
}

/// Diff live tables against SCHEMA_SQL and ADD any missing columns.
// PARITY: hermes_state_schema.py _reconcile_columns @ b9aa928
pub fn reconcile_columns(conn: &Connection) -> rusqlite::Result<()> {
    let expected = parse_schema_columns(SCHEMA_SQL);
    for (table_name, declared_cols) in expected {
        let live: Vec<String> = match conn.prepare(&format!(
            "PRAGMA table_info(\"{}\")",
            table_name.replace('"', "\"\"")
        )) {
            Ok(mut stmt) => stmt
                .query_map([], |r| r.get::<_, String>(1))
                .and_then(|m| m.collect())?,
            Err(rusqlite::Error::SqliteFailure(_, _)) => continue, // table absent
            Err(e) => return Err(e),
        };
        for (col_name, col_type) in declared_cols {
            if live.contains(&col_name) {
                continue;
            }
            let safe_name = col_name.replace('"', "\"\"");
            // Duplicate-column races and NOT NULL-without-default mistakes are
            // logged (debug) upstream and tolerated.
            let _ = conn.execute_batch(&format!(
                "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
                table_name.replace('"', "\"\""),
                safe_name,
                col_type
            ));
        }
    }
    Ok(())
}

/// Number of FTS triggers present.
// PARITY: hermes_state_schema.py _fts_trigger_count @ b9aa928
pub fn fts_trigger_count(conn: &Connection) -> rusqlite::Result<i64> {
    let placeholders: Vec<String> = (0.._FTS_TRIGGERS.len()).map(|_| "?".to_string()).collect();
    let params = rusqlite::params_from_iter(_FTS_TRIGGERS.iter());
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name IN ({})",
            placeholders.join(",")
        ),
        params,
        |r| r.get(0),
    )
}

/// True when trigger SQL is missing `AFTER UPDATE OF` (still broad).
// PARITY: hermes_state_schema.py _fts_update_trigger_needs_narrowing @ b9aa928
pub fn fts_update_trigger_needs_narrowing(sql: Option<&str>) -> bool {
    let Some(sql) = sql else { return false };
    let compact = sql.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase();
    if compact.contains("AFTER UPDATE OF ") {
        return false;
    }
    compact.contains("AFTER UPDATE ON ")
}

/// Replace broad `AFTER UPDATE` FTS triggers with `AFTER UPDATE OF` variants.
// PARITY: hermes_state_schema.py _migrate_broad_fts_update_triggers @ b9aa928
#[allow(clippy::redundant_closure_call)]
pub fn migrate_broad_fts_update_triggers(db: &SessionDB) -> i64 {
    let conn = &*db.writer_conn();
    let legacy = db_has_legacy_inline_fts(conn).unwrap_or(false);
    let mut update_names: Vec<&str> = vec!["messages_fts_update", "messages_fts_trigram_update"];
    if !legacy {
        update_names.push("messages_fts_cjk_update");
    }
    let placeholders: Vec<String> = (0..update_names.len()).map(|_| "?".to_string()).collect();
    let params = rusqlite::params_from_iter(update_names.iter());
    let rows = match conn.prepare(&format!(
        "SELECT name, sql FROM sqlite_master WHERE type = 'trigger' AND name IN ({})",
        placeholders.join(",")
    )) {
        Ok(mut stmt) => stmt
            .query_map(params, |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
            .and_then(|m| m.collect())
            .unwrap_or_default(),
        Err(_) => vec![],
    };
    let mut to_drop: Vec<String> = Vec::new();
    for (name, sql) in rows {
        if fts_update_trigger_needs_narrowing(sql.as_deref()) {
            to_drop.push(name);
        }
    }
    if to_drop.is_empty() {
        return 0;
    }
    for name in &to_drop {
        let _ = conn.execute_batch(&format!("DROP TRIGGER IF EXISTS {}", name));
    }
    if legacy {
        let _ = db.ensure_fts_schema(conn, "messages_fts", LEGACY_FTS_SQL);
        let _ = db.ensure_fts_schema(conn, "messages_fts_trigram", LEGACY_FTS_TRIGRAM_SQL);
    } else {
        let _ = db.ensure_fts_schema(conn, "messages_fts", FTS_SQL);
        let _ = db.ensure_fts_schema(conn, "messages_fts_trigram", FTS_TRIGRAM_SQL);
        if to_drop.iter().any(|n| n == "messages_fts_cjk_update") {
            // CJK re-ensure; on any doubt, quarantine (fail-closed).
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                db.ensure_fts_cjk_schema(conn);
            }));
            if result.is_err() || !db.cjk_update_trigger_is_narrowed(conn) {
                db.quarantine_cjk_after_update_of_migration(conn);
                eprintln!(
                    "CJK FTS UPDATE trigger missing or still broad after UPDATE OF migration; marked stale and unavailable"
                );
            }
        }
    }
    eprintln!(
        "Migrated {} broad FTS UPDATE trigger(s) to AFTER UPDATE OF (no rebuild required)",
        to_drop.len()
    );
    to_drop.len() as i64
}

/// True when `messages_fts_cjk_update` exists with `AFTER UPDATE OF`.
// PARITY: hermes_state_schema.py _cjk_update_trigger_is_narrowed @ b9aa928
pub fn cjk_update_trigger_is_narrowed(conn: &Connection) -> bool {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'messages_fts_cjk_update'",
            [],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    match sql {
        Some(s) => !fts_update_trigger_needs_narrowing(Some(&s)),
        None => false,
    }
}

/// Probe an FTS virtual table: Some(true) exists, Some(false) absent,
/// None capability-unavailable.
///
/// rusqlite attaches the SQLite error message at *prepare* time, so the
/// probe compiles `SELECT ... LIMIT 0` and inspects the prepare error —
/// exactly the "column resolution happens at prepare time" contract the
/// upstream cursor probe relies on.
// PARITY: hermes_state_schema.py _fts_table_probe @ b9aa928
pub fn fts_table_probe(conn: &Connection, table_name: &str) -> Option<bool> {
    let sql = format!("SELECT * FROM {} LIMIT 0", table_name);
    match conn.prepare(&sql) {
        Ok(_stmt) => Some(true),
        Err(rusqlite::Error::SqliteFailure(_, Some(msg))) => {
            if is_fts5_unavailable_error(&msg) {
                return None;
            }
            if msg.to_ascii_lowercase().contains("no such table") {
                return Some(false);
            }
            None
        }
        Err(rusqlite::Error::SqliteFailure(_, None)) => None,
        Err(_) => None,
    }
}

/// Rebuild both v23 external-content FTS indexes.
// PARITY: hermes_state_schema.py _rebuild_fts_indexes @ b9aa928
pub fn rebuild_fts_indexes(conn: &Connection, include_trigram: bool) -> rusqlite::Result<()> {
    conn.execute_batch("INSERT INTO messages_fts(messages_fts) VALUES('rebuild')")?;
    if include_trigram {
        conn.execute_batch(
            "INSERT INTO messages_fts_trigram(messages_fts_trigram) VALUES('rebuild')",
        )?;
    }
    conn.execute_batch(
        "DELETE FROM state_meta WHERE key IN ('fts_rebuild_high_water', 'fts_rebuild_progress')",
    )?;
    Ok(())
}

/// Rebuild the LEGACY inline FTS indexes (pre-v23) from messages.
// PARITY: hermes_state_schema.py _rebuild_legacy_fts_indexes @ b9aa928
pub fn rebuild_legacy_fts_indexes(conn: &Connection, include_trigram: bool) -> rusqlite::Result<()> {
    conn.execute_batch("DELETE FROM messages_fts")?;
    conn.execute_batch(
        "INSERT INTO messages_fts(rowid, content) \
         SELECT id, COALESCE(content, '') || ' ' || COALESCE(tool_name, '') || ' ' || COALESCE(tool_calls, '') \
         FROM messages",
    )?;
    if include_trigram {
        conn.execute_batch("DELETE FROM messages_fts_trigram")?;
        conn.execute_batch(
            "INSERT INTO messages_fts_trigram(rowid, content) \
             SELECT id, COALESCE(content, '') || ' ' || COALESCE(tool_name, '') || ' ' || COALESCE(tool_calls, '') \
             FROM messages",
        )?;
    }
    Ok(())
}

/// True when demoted v22 shadow tables are still awaiting teardown.
// PARITY: hermes_state.py _has_fts_trash @ b9aa928 (_FTS_TRASH_PREFIX = "fts_v22_trash_")
pub fn has_fts_trash(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name LIKE ? ESCAPE '\\' LIMIT 1",
        [format!("{}%", "fts_v22_trash_".replace('_', "\\_"))],
        |_r| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

/// True when the base FTS table exists but indexes nothing while `messages`
/// has rows.
// PARITY: hermes_state_search.py _fts_external_index_empty_with_messages @ b9aa928
pub fn fts_external_index_empty_with_messages(conn: &Connection) -> bool {
    match conn.query_row("SELECT EXISTS(SELECT 1 FROM messages)", [], |r| {
        r.get::<_, i64>(0)
    }) {
        Ok(0) | Err(_) => return false,
        Ok(_) => {}
    }
    match conn.query_row("SELECT EXISTS(SELECT 1 FROM messages_fts_docsize)", [], |r| {
        r.get::<_, i64>(0)
    }) {
        Ok(0) => true,
        Ok(_) => false,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// impl SessionDB methods (the schema-cluster surface)
// ---------------------------------------------------------------------------

impl SessionDB {
    /// One-time `_dedupe_legacy_system_prompts`.
    // PARITY: hermes_state_schema.py @ b9aa928
    pub(crate) fn dedupe_legacy_system_prompts(&self, conn: &Connection) {
        let rows: Vec<(String, String)> = match conn.prepare(
            "SELECT id, system_prompt FROM sessions WHERE system_prompt IS NOT NULL",
        ) {
            Ok(mut stmt) => stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .and_then(|m| m.collect())
                .unwrap_or_default(),
            Err(_) => return,
        };
        for (session_id, prompt) in rows {
            let prompt_hash = store_system_prompt(conn, Some(prompt)).unwrap_or_default();
            let _ = conn.execute(
                "UPDATE sessions SET system_prompt_hash = ?, system_prompt = NULL WHERE id = ?",
                rusqlite::params![prompt_hash, session_id],
            );
        }
    }

    /// `_sqlite_supports_fts5`
    // PARITY: hermes_state_schema.py @ b9aa928
    pub(crate) fn sqlite_supports_fts5(&self, conn: &Connection) -> bool {
        match conn.execute_batch("CREATE VIRTUAL TABLE temp._hermes_fts5_probe USING fts5(x)") {
            Ok(()) => {
                let _ = conn.execute_batch("DROP TABLE temp._hermes_fts5_probe");
                true
            }
            Err(rusqlite::Error::SqliteFailure(e, _)) => {
                let msg = e.to_string();
                if is_fts5_unavailable_error(&msg) {
                    self.warn_fts5_unavailable(&msg);
                    return false;
                }
                // Capability-surface probe is best-effort; non-capability
                // failures propagate as unsupported (upstream re-raises).
                false
            }
            Err(_) => false,
        }
    }

    /// `_warn_fts5_unavailable`
    // PARITY: hermes_state.py @ b9aa928
    pub(crate) fn warn_fts5_unavailable(&self, exc: &str) {
        self.set_fts_enabled(false);
        if self.fts_unavailable_warned() {
            return;
        }
        self.set_fts_unavailable_warned(true);
        eprintln!(
            "SQLite FTS5 unavailable for {}; full-text session search disabled. Run `hermes update` to rebuild the venv with a current Python (managed uv guarantees FTS5). (underlying error: {})",
            self.db_path.display(),
            exc,
        );
    }

    /// `_warn_trigram_unavailable`
    // PARITY: hermes_state.py @ b9aa928
    pub(crate) fn warn_trigram_unavailable(&self, exc: &str) {
        if self.trigram_unavailable_warned() {
            return;
        }
        self.set_trigram_unavailable_warned(true);
        eprintln!(
            "SQLite trigram tokenizer unavailable for {} (requires SQLite >= 3.34, this build is {}); CJK/substring search will fall back to LIKE: {}",
            self.db_path.display(),
            rusqlite::version(),
            exc,
        );
    }

    /// `_ensure_fts_cjk_schema` (never raises; every failure degrades to
    /// "no cjk index").
    // PARITY: hermes_state.py @ b9aa928
    pub(crate) fn ensure_fts_cjk_schema(&self, conn: &Connection) {
        let cjk_present = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'messages_fts_cjk'",
                [],
                |_r| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();

        if !self.fts_cjk_loaded() {
            if cjk_present {
                let live: Vec<String> = conn
                    .prepare(&format!(
                        "SELECT name FROM sqlite_master WHERE type = 'trigger' \
                         AND name IN ({})",
                        (0.._FTS_CJK_TRIGGERS.len())
                            .map(|_| "?")
                            .collect::<Vec<_>>()
                            .join(",")
                    ))
                    .ok()
                    .and_then(|mut stmt| {
                        stmt.query_map(
                            rusqlite::params_from_iter(_FTS_CJK_TRIGGERS.iter()),
                            |r| r.get::<_, String>(0),
                        )
                        .ok()
                        .map(|it| it.filter_map(|r| r.ok()).collect::<Vec<_>>())
                    })
                    .unwrap_or_default();
                if !live.is_empty() {
                    eprintln!(
                        "messages_fts_cjk triggers present but the cjk_unicode61 tokenizer is unavailable ({}) — dropping the cjk triggers so message writes keep working. CJK search falls back to trigram/LIKE; run `hermes sessions optimize-storage` on a host with the extension to rebuild.",
                        crate::state::fts5_cjk_so_path().display(),
                    );
                    let _ = conn.execute(
                        "INSERT INTO state_meta (key, value) VALUES (?, '1') \
                         ON CONFLICT(key) DO UPDATE SET value = '1'",
                        [FTS_CJK_STALE_KEY],
                    );
                    for trig in &live {
                        let _ = conn.execute_batch(&format!("DROP TRIGGER IF EXISTS {}", trig));
                    }
                }
            }
            self.set_fts_cjk_available(false);
            return;
        }

        if conn.execute_batch(FTS_CJK_TABLE_SQL).is_err() {
            eprintln!("messages_fts_cjk ensure failed; CJK search stays on trigram/LIKE");
            self.set_fts_cjk_available(false);
            return;
        }
        if !cjk_present {
            let _ = conn.execute("DELETE FROM state_meta WHERE key = ?", [FTS_CJK_STALE_KEY]);
            let n_msgs: i64 = conn
                .query_row("SELECT COUNT(*) FROM messages WHERE role <> 'tool'", [], |r| {
                    r.get(0)
                })
                .unwrap_or(0);
            if n_msgs > 0 {
                let hw: i64 = conn
                    .query_row("SELECT COALESCE(MAX(id), 0) FROM messages", [], |r| r.get(0))
                    .unwrap_or(0);
                for (k, v) in [
                    ("fts_cjk_rebuild_high_water", hw.to_string()),
                    ("fts_cjk_rebuild_progress", "0".to_string()),
                ] {
                    let _ = conn.execute(
                        "INSERT INTO state_meta (key, value) VALUES (?, ?) \
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        rusqlite::params![k, v],
                    );
                }
            }
        }
        let stale = conn
            .query_row("SELECT 1 FROM state_meta WHERE key = ?", [FTS_CJK_STALE_KEY], |_r| {
                Ok(())
            })
            .optional()
            .ok()
            .flatten()
            .is_some();
        if stale {
            self.set_fts_cjk_available(false);
            return;
        }
        if conn.execute_batch(FTS_CJK_TRIGGER_SQL).is_err() {
            eprintln!("messages_fts_cjk ensure failed; CJK search stays on trigram/LIKE");
            self.set_fts_cjk_available(false);
            return;
        }
        let backfill_pending = conn
            .query_row(
                "SELECT 1 FROM state_meta WHERE key = 'fts_cjk_rebuild_high_water' LIMIT 1",
                [],
                |_r| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();
        self.set_fts_cjk_available(!backfill_pending);
    }

    /// `_drop_fts_triggers`
    // PARITY: hermes_state.py @ b9aa928
    pub(crate) fn drop_fts_triggers(&self, conn: &Connection) {
        for trigger in _FTS_TRIGGERS {
            let _ = conn.execute_batch(&format!("DROP TRIGGER IF EXISTS {}", trigger));
        }
    }

    /// `_ensure_fts_schema`
    // PARITY: hermes_state.py @ b9aa928
    pub(crate) fn ensure_fts_schema(&self, conn: &Connection, table_name: &str, ddl: &str) -> bool {
        if fts_table_probe(conn, table_name).is_none() {
            self.warn_fts5_unavailable("no such module: fts5");
            return false;
        }
        // Run even when the virtual table exists so any dropped or missing
        // triggers are recreated after a previous no-FTS5 runtime disabled
        // them to keep message writes working.
        match conn.execute_batch(ddl) {
            Ok(()) => true,
            Err(rusqlite::Error::SqliteFailure(e, _)) => {
                let msg = e.to_string();
                if !is_fts5_unavailable_error(&msg) {
                    return false;
                }
                if is_trigram_unavailable_error(&msg) {
                    self.warn_trigram_unavailable(&msg);
                } else {
                    self.warn_fts5_unavailable(&msg);
                }
                false
            }
            Err(_) => false,
        }
    }
}

/// Store a system prompt (content-addressed), returning its hash.
// PARITY: hermes_state.py _store_system_prompt + _system_prompt_hash @ b9aa928
pub fn store_system_prompt(
    conn: &Connection,
    system_prompt: Option<String>,
) -> rusqlite::Result<Option<String>> {
    let Some(prompt) = system_prompt else { return Ok(None) };
    let prompt_hash = system_prompt_hash(&prompt);
    conn.execute(
        "INSERT OR IGNORE INTO system_prompts (hash, prompt) VALUES (?, ?)",
        rusqlite::params![prompt_hash, prompt],
    )?;
    Ok(Some(prompt_hash))
}

/// sha256 hexdigest — `hashlib.sha256(...).hexdigest()`.
// PARITY: hermes_state.py _system_prompt_hash @ b9aa928
pub fn system_prompt_hash(system_prompt: &str) -> String {
    let digest = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(system_prompt.as_bytes());
        h.finalize()
    };
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// _init_schema — the full schema bootstrap
// ---------------------------------------------------------------------------

impl SessionDB {
    /// Create tables and FTS if they don't exist, reconcile columns, run
    /// version-gated data migrations, and stamp schema_version.
    ///
    /// PARITY: hermes_state_schema.py _init_schema @ b9aa928 (619–1078).
    /// The v10 trigram backfill branch is unreachable at SCHEMA_VERSION 25
    /// (upstream keeps it for archaeology: `current_version < 10 and
    /// SCHEMA_VERSION == 10`); it is not reproduced.
    #[allow(clippy::redundant_closure_call)]
    pub(crate) fn init_schema_inner(&self, conn: &Connection) -> Result<(), String> {
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;

        // Declarative column reconciliation.
        reconcile_columns(conn).map_err(|e| e.to_string())?;

        // PK-shape repairs reconciliation cannot express.
        heal_gateway_routing_pk(conn).map_err(|e| e.to_string())?;
        heal_session_model_usage_pk(conn).map_err(|e| e.to_string())?;

        // Index referencing the reconciler-added platform_message_id column.
        let _ = conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_messages_platform_msg_id \
             ON messages(session_id, platform_message_id) WHERE platform_message_id IS NOT NULL",
        );

        // Deferred indexes referencing the reconciler-added active column.
        conn.execute_batch(DEFERRED_INDEX_SQL).map_err(|e| e.to_string())?;

        // Heal NULL active rows unconditionally on every startup.
        let _ = conn.execute_batch("UPDATE messages SET active = 1 WHERE active IS NULL");

        let fts5_available = self.sqlite_supports_fts5(conn);
        let fts_migrations_complete = true;
        if !fts5_available {
            // Drop only the triggers so core persistence continues; if a future
            // runtime has FTS5, ensure_fts_schema recreates them.
            self.drop_fts_triggers(conn);
        }

        // ── Schema version bookkeeping ────────────────────────────────────
        let row: Option<i64> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        match row {
            None => {
                conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?)",
                    [crate::common::SCHEMA_VERSION],
                )
                .map_err(|e| e.to_string())?;
            }
            Some(current_version) => {
                if current_version < 16 && crate::common::SCHEMA_VERSION >= 16 {
                    // v16: tag delegate subagent rows.
                    let _ = conn.execute_batch(&format!(
                        "UPDATE sessions SET model_config = json_set(COALESCE(model_config, '{{}}'), '$._delegate_from', parent_session_id) \
                         WHERE parent_session_id IS NOT NULL \
                         AND json_extract(COALESCE(model_config, '{{}}'), '$._delegate_from') IS NULL \
                         AND {}",
                        crate::common::_ephemeral_child_sql("sessions")
                    ));
                    let _ = conn.execute_batch(
                        "UPDATE sessions SET model_config = json_set(COALESCE(model_config, '{}'), '$._delegate_from', '__orphaned__') \
                         WHERE parent_session_id IS NULL \
                         AND json_extract(COALESCE(model_config, '{}'), '$._delegate_from') IS NULL \
                         AND json_extract(COALESCE(model_config, '{}'), '$._branched_from') IS NULL \
                         AND title IS NULL \
                         AND message_count <= 25 \
                         AND EXISTS (SELECT 1 FROM messages m WHERE m.session_id = sessions.id AND m.role = 'tool') \
                         AND NOT EXISTS (SELECT 1 FROM sessions ch WHERE ch.parent_session_id = sessions.id)",
                    );
                }
                if current_version < 18 && crate::common::SCHEMA_VERSION >= 18 {
                    // v18: gateway metadata consolidation — best-effort.
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || self.backfill_gateway_metadata_from_sessions_json(conn),
                    ))
                    .is_err()
                    {
                        // best-effort backfill; failures degrade silently
                    }
                }
                if current_version < 20 && crate::common::SCHEMA_VERSION >= 20 {
                    // v20: per-model usage attribution seed.
                    let _ = conn.execute_batch(
                        "INSERT OR IGNORE INTO session_model_usage ( \
                             session_id, model, billing_provider, billing_base_url, billing_mode, \
                             api_call_count, input_tokens, output_tokens, cache_read_tokens, \
                             cache_write_tokens, reasoning_tokens, estimated_cost_usd, actual_cost_usd, \
                             cost_status, cost_source, first_seen, last_seen \
                         ) \
                         SELECT id, COALESCE(model, 'unknown'), COALESCE(billing_provider, ''), \
                                COALESCE(billing_base_url, ''), COALESCE(billing_mode, ''), \
                                COALESCE(api_call_count, 0), COALESCE(input_tokens, 0), \
                                COALESCE(output_tokens, 0), COALESCE(cache_read_tokens, 0), \
                                COALESCE(cache_write_tokens, 0), COALESCE(reasoning_tokens, 0), \
                                COALESCE(estimated_cost_usd, 0), COALESCE(actual_cost_usd, 0), \
                                cost_status, cost_source, started_at, COALESCE(ended_at, started_at) \
                         FROM sessions \
                         WHERE COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0) \
                               + COALESCE(cache_read_tokens, 0) + COALESCE(cache_write_tokens, 0) \
                               + COALESCE(reasoning_tokens, 0) > 0",
                    );
                }
                if current_version < 22 && crate::common::SCHEMA_VERSION >= 22 {
                    // v22: task-dimension usage attribution (PK rebuild).
                    let legacy_pk: Option<i64> = conn
                        .query_row(
                            "SELECT COUNT(*) FROM pragma_table_info('session_model_usage') WHERE name = 'task' AND pk > 0",
                            [],
                            |r| r.get(0),
                        )
                        .optional()
                        .ok()
                        .flatten();
                    if legacy_pk == Some(0) || legacy_pk.is_none() {
                        let _ = conn.execute_batch(
                            "ALTER TABLE session_model_usage RENAME TO session_model_usage_v21; \
                             CREATE TABLE session_model_usage ( \
                                 session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,\n\
                                 model TEXT NOT NULL,\n\
                                 billing_provider TEXT NOT NULL DEFAULT '',\n\
                                 billing_base_url TEXT NOT NULL DEFAULT '',\n\
                                 billing_mode TEXT NOT NULL DEFAULT '',\n\
                                 task TEXT NOT NULL DEFAULT '',\n\
                                 api_call_count INTEGER NOT NULL DEFAULT 0,\n\
                                 input_tokens INTEGER NOT NULL DEFAULT 0,\n\
                                 output_tokens INTEGER NOT NULL DEFAULT 0,\n\
                                 cache_read_tokens INTEGER NOT NULL DEFAULT 0,\n\
                                 cache_write_tokens INTEGER NOT NULL DEFAULT 0,\n\
                                 reasoning_tokens INTEGER NOT NULL DEFAULT 0,\n\
                                 estimated_cost_usd REAL NOT NULL DEFAULT 0,\n\
                                 actual_cost_usd REAL NOT NULL DEFAULT 0,\n\
                                 cost_status TEXT,\n\
                                 cost_source TEXT,\n\
                                 first_seen REAL,\n\
                                 last_seen REAL,\n\
                                 PRIMARY KEY (session_id, model, billing_provider, billing_base_url, billing_mode, task)\n\
                             );\n\
                             INSERT INTO session_model_usage ( \
                                 session_id, model, billing_provider, billing_base_url, \
                                 billing_mode, task, api_call_count, input_tokens, \
                                 output_tokens, cache_read_tokens, cache_write_tokens, \
                                 reasoning_tokens, estimated_cost_usd, actual_cost_usd, \
                                 cost_status, cost_source, first_seen, last_seen \
                             ) \
                             SELECT session_id, model, billing_provider, billing_base_url, \
                                    billing_mode, '', api_call_count, input_tokens, \
                                    output_tokens, cache_read_tokens, cache_write_tokens, \
                                    reasoning_tokens, estimated_cost_usd, actual_cost_usd, \
                                    cost_status, cost_source, first_seen, last_seen \
                             FROM session_model_usage_v21; \
                             DROP TABLE session_model_usage_v21; \
                             CREATE INDEX IF NOT EXISTS idx_session_model_usage_session ON session_model_usage(session_id); \
                             CREATE INDEX IF NOT EXISTS idx_session_model_usage_model ON session_model_usage(model)",
                        );
                    }
                }
                if current_version < 23 && crate::common::SCHEMA_VERSION >= 23 {
                    // v23: FTS storage redesign is OPT-IN; flag availability on
                    // legacy installs.
                    if fts5_available && db_has_legacy_inline_fts(conn).unwrap_or(false) {
                        let _ = self.set_meta_cursor(conn, "fts_optimize_available", "1");
                    }
                }
                if current_version < 25 && crate::common::SCHEMA_VERSION >= 25 {
                    // v25: de-duplicate per-session system prompt snapshots.
                    self.dedupe_legacy_system_prompts(conn);
                }

                // Stamp the FTS storage layout (independent of schema_version).
                let stamp = fts5_available
                    && !db_has_legacy_inline_fts(conn).unwrap_or(true)
                    && conn
                        .query_row(
                            "SELECT 1 FROM state_meta WHERE key = 'fts_rebuild_high_water' LIMIT 1",
                            [],
                            |_r| Ok(()),
                        )
                        .optional()
                        .ok()
                        .flatten()
                        .is_none()
                    && !has_fts_trash(conn)
                    && !fts_external_index_empty_with_messages(conn);
                if stamp {
                    let _ = self.set_meta_cursor(
                        conn,
                        "fts_storage_version",
                        &crate::common::FTS_STORAGE_VERSION.to_string(),
                    );
                }

                // Advance schema_version (only when FTS migrations are
                // complete and FTS5 is available — else claiming current
                // schema would be a lie).
                if current_version < crate::common::SCHEMA_VERSION
                    && fts_migrations_complete
                    && fts5_available
                {
                    conn.execute(
                        "UPDATE schema_version SET version = ?",
                        [crate::common::SCHEMA_VERSION],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        // Unique title index — always ensure it exists.
        let title_index_sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_title_unique \
             ON sessions(title) WHERE title IS NOT NULL";
        match conn.execute_batch(title_index_sql) {
            Ok(()) => {}
            Err(_) => {
                // IntegrityError family — repair duplicates, retry once.
                let _ = conn.execute_batch(
                    "UPDATE sessions AS older \
                     SET title = NULL \
                     WHERE title IS NOT NULL \
                       AND EXISTS (SELECT 1 FROM sessions AS newer \
                                   WHERE newer.title = older.title AND newer.rowid > older.rowid)",
                );
                let _ = conn.execute_batch(title_index_sql);
            }
        }

        if fts5_available {
            if db_has_legacy_inline_fts(conn).unwrap_or(false) {
                let triggers_need_repair = fts_trigger_count(conn).unwrap_or(0) < _FTS_TRIGGERS.len() as i64;
                let enabled = self.ensure_fts_schema(conn, "messages_fts", LEGACY_FTS_SQL);
                self.set_fts_enabled(enabled);
                if enabled {
                    let trigram_enabled = self.ensure_fts_schema(conn, "messages_fts_trigram", LEGACY_FTS_TRIGRAM_SQL);
                    self.set_trigram_available(trigram_enabled);
                    if triggers_need_repair {
                        let _ = rebuild_legacy_fts_indexes(conn, trigram_enabled);
                    }
                }
            } else {
                let triggers_need_repair = fts_trigger_count(conn).unwrap_or(0) < _FTS_TRIGGERS.len() as i64;
                let enabled = self.ensure_fts_schema(conn, "messages_fts", FTS_SQL);
                self.set_fts_enabled(enabled);
                if enabled {
                    let trigram_enabled = self.ensure_fts_schema(conn, "messages_fts_trigram", FTS_TRIGRAM_SQL);
                    self.set_trigram_available(trigram_enabled);
                    if triggers_need_repair {
                        let _ = rebuild_fts_indexes(conn, trigram_enabled);
                    }
                    // CJK-bigram index (strictly additive; gated on the
                    // loadable tokenizer).
                    self.ensure_fts_cjk_schema(conn);
                }
            }

            // Replace any pre-existing broad AFTER UPDATE triggers.
            if self.fts_enabled() {
                migrate_broad_fts_update_triggers(self);
            }
        }

        Ok(())
    }

    /// One-time v18 backfill of gateway metadata from sessions.json.
    // PARITY: hermes_state_schema.py _backfill_gateway_metadata_from_sessions_json @ b9aa928
    pub(crate) fn backfill_gateway_metadata_from_sessions_json(&self, conn: &Connection) {
        let sessions_file = hermes_constants::get_hermes_home().join("sessions").join("sessions.json");
        if !sessions_file.exists() {
            return;
        }
        let Ok(data) = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&sessions_file).unwrap_or_default(),
        ) else {
            return;
        };
        let Some(obj) = data.as_object() else { return };
        for (key, entry) in obj {
            if key.starts_with('_') {
                continue;
            }
            let Some(entry) = entry.as_object() else { continue };
            let Some(session_id) = entry.get("session_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let origin = entry.get("origin").and_then(|v| v.as_object());
            let _ = conn.execute(
                "UPDATE sessions \
                 SET session_key = COALESCE(session_key, ?), \
                     chat_id = COALESCE(chat_id, ?), \
                     chat_type = COALESCE(chat_type, ?), \
                     thread_id = COALESCE(thread_id, ?), \
                     display_name = COALESCE(display_name, ?), \
                     origin_json = COALESCE(origin_json, ?), \
                     expiry_finalized = CASE WHEN COALESCE(expiry_finalized, 0) = 0 AND ? = 1 THEN 1 ELSE expiry_finalized END \
                 WHERE id = ?",
                rusqlite::params![
                    entry.get("session_key").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_else(|| key.clone()),
                    origin.and_then(|o| o.get("chat_id")).and_then(|v| v.as_str()),
                    entry.get("chat_type").and_then(|v| v.as_str()),
                    origin.and_then(|o| o.get("thread_id")).and_then(|v| v.as_str()),
                    entry.get("display_name").and_then(|v| v.as_str()),
                    origin.map(|o| serde_json::json!(o).to_string()),
                    {
                        let ef = entry.get("expiry_finalized").and_then(|v| v.as_bool()).unwrap_or(false);
                        let mf = entry.get("memory_flushed").and_then(|v| v.as_bool()).unwrap_or(false);
                        if ef || mf { 1 } else { 0 }
                    },
                    session_id,
                ],
            );
        }
    }
}

/// Rebuild `gateway_routing` when its PRIMARY KEY predates scoping.
// PARITY: hermes_state_schema.py _heal_gateway_routing_pk @ b9aa928
pub fn heal_gateway_routing_pk(conn: &Connection) -> rusqlite::Result<()> {
    let rows: Vec<(String, i64)> = match conn.prepare("PRAGMA table_info(\"gateway_routing\")") {
        Ok(mut stmt) => stmt
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))
            .and_then(|m| m.collect())?,
        Err(_) => return Ok(()),
    };
    if rows.is_empty() {
        return Ok(());
    }
    let mut pk_cols: Vec<&str> = rows
        .iter()
        .filter(|(_, pk)| *pk > 0)
        .map(|(name, _)| name.as_str())
        .collect();
    pk_cols.sort();
    if pk_cols == vec!["scope", "session_key"] {
        return Ok(());
    }
    eprintln!("gateway_routing has legacy primary key {:?}; rebuilding with composite (scope, session_key) key", pk_cols);
    conn.execute_batch(
        "ALTER TABLE gateway_routing RENAME TO gateway_routing_legacy_pk; \
         CREATE TABLE gateway_routing (\n\
             scope TEXT NOT NULL DEFAULT '',\n\
             session_key TEXT NOT NULL,\n\
             entry_json TEXT NOT NULL,\n\
             updated_at REAL NOT NULL,\n\
             PRIMARY KEY (scope, session_key)\n\
         );\n\
         INSERT OR REPLACE INTO gateway_routing (scope, session_key, entry_json, updated_at) \
         SELECT COALESCE(scope, ''), session_key, entry_json, updated_at \
         FROM gateway_routing_legacy_pk ORDER BY updated_at ASC; \
         DROP TABLE gateway_routing_legacy_pk",
    )
}

/// Rebuild `session_model_usage` when its PRIMARY KEY lacks `task`.
// PARITY: hermes_state_schema.py _heal_session_model_usage_pk @ b9aa928
pub fn heal_session_model_usage_pk(conn: &Connection) -> rusqlite::Result<()> {
    let rows: Vec<(String, i64)> = match conn.prepare("PRAGMA table_info(\"session_model_usage\")") {
        Ok(mut stmt) => stmt
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))
            .and_then(|m| m.collect())?,
        Err(_) => return Ok(()),
    };
    if rows.is_empty() {
        // Table doesn't exist yet — SCHEMA_SQL creates it correctly.
        return Ok(());
    }
    if rows.iter().any(|(name, pk)| *pk > 0 && name == "task") {
        return Ok(());
    }
    eprintln!(
        "session_model_usage has legacy primary key (missing task); rebuilding with composite 6-column key"
    );
    // FK-off window (mirrors upstream): _init_schema runs outside a
    // transaction, so PRAGMA foreign_keys is effective here.
    conn.execute_batch("PRAGMA foreign_keys=OFF")?;
    #[allow(clippy::redundant_closure_call)]
    let result = (|| -> rusqlite::Result<()> {
        conn.execute_batch(
            "ALTER TABLE session_model_usage RENAME TO session_model_usage_legacy_pk; \
             CREATE TABLE session_model_usage (\n\
                 session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,\n\
                 model TEXT NOT NULL,\n\
                 billing_provider TEXT NOT NULL DEFAULT '',\n\
                 billing_base_url TEXT NOT NULL DEFAULT '',\n\
                 billing_mode TEXT NOT NULL DEFAULT '',\n\
                 task TEXT NOT NULL DEFAULT '',\n\
                 api_call_count INTEGER NOT NULL DEFAULT 0,\n\
                 input_tokens INTEGER NOT NULL DEFAULT 0,\n\
                 output_tokens INTEGER NOT NULL DEFAULT 0,\n\
                 cache_read_tokens INTEGER NOT NULL DEFAULT 0,\n\
                 cache_write_tokens INTEGER NOT NULL DEFAULT 0,\n\
                 reasoning_tokens INTEGER NOT NULL DEFAULT 0,\n\
                 estimated_cost_usd REAL NOT NULL DEFAULT 0,\n\
                 actual_cost_usd REAL NOT NULL DEFAULT 0,\n\
                 cost_status TEXT,\n\
                 cost_source TEXT,\n\
                 first_seen REAL,\n\
                 last_seen REAL,\n\
                 PRIMARY KEY (session_id, model, billing_provider, billing_base_url, billing_mode, task)\n\
             );\n\
             INSERT OR IGNORE INTO session_model_usage (\n\
                 session_id, model, billing_provider, billing_base_url,\n\
                 billing_mode, task, api_call_count, input_tokens,\n\
                 output_tokens, cache_read_tokens, cache_write_tokens,\n\
                 reasoning_tokens, estimated_cost_usd, actual_cost_usd,\n\
                 cost_status, cost_source, first_seen, last_seen\n\
             )\n\
             SELECT session_id, model, COALESCE(billing_provider, ''), COALESCE(billing_base_url, ''),\n\
                    COALESCE(billing_mode, ''), COALESCE(task, ''), api_call_count, input_tokens,\n\
                    output_tokens, cache_read_tokens, cache_write_tokens,\n\
                    reasoning_tokens, estimated_cost_usd, actual_cost_usd,\n\
                    cost_status, cost_source, first_seen, last_seen\n\
             FROM session_model_usage_legacy_pk; \n\
             DROP TABLE session_model_usage_legacy_pk; \n\
             CREATE INDEX IF NOT EXISTS idx_session_model_usage_session ON session_model_usage(session_id); \n\
             CREATE INDEX IF NOT EXISTS idx_session_model_usage_model ON session_model_usage(model)",
        )
    })();
    let _ = conn.execute_batch("PRAGMA foreign_keys=ON");
    result
}


/// Test-only probe wrapper.
pub fn ensure_fts_schema_probe(conn: &Connection, table_name: &str) -> Option<bool> {
    fts_table_probe(conn, table_name)
}
