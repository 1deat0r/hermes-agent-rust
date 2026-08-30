//! Shared SQLite primitives for the small per-profile / board stores.
//!
//! PARITY: `hermes_cli/sqlite_util.py` @ b9aa928 (whole module).
//!
//! The projects and kanban stores open WAL SQLite files with the same two
//! primitives — an idempotent column-add migration and an IMMEDIATE write
//! transaction. One definition here keeps the two stores from drifting.

use rusqlite::Connection;

/// `ALTER TABLE <table> ADD COLUMN <ddl>`, idempotent across races.
///
/// Returns `Ok(true)` when this call added the column. Swallows the
/// `duplicate column name` error a concurrent migrator may have run first
/// (issue #21708). `column` is the human-readable name for the call site;
/// `ddl` carries the actual definition.
///
/// PARITY: `add_column_if_missing` (upstream lines 12-27).
pub fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    ddl: &str,
) -> rusqlite::Result<bool> {
    let _ = column;
    match conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {ddl}"), []) {
        Ok(_) => Ok(true),
        Err(err) => {
            // `sqlite3.OperationalError` with "duplicate column name" → the
            // concurrent migrator won; report false instead of raising.
            let text = err.to_string().to_lowercase();
            if text.contains("duplicate column name") {
                Ok(false)
            } else {
                Err(err)
            }
        }
    }
}

/// An IMMEDIATE write transaction: at most one concurrent writer wins.
///
/// PARITY: `write_txn` (upstream lines 30-48) — the Python
/// `@contextmanager` becomes a closure form: `BEGIN IMMEDIATE`, run the
/// body, then `COMMIT`; on a body error the guarded `ROLLBACK` runs (a
/// rollback failure — e.g. SQLite's auto-rollback under EIO / lock
/// contention / corruption leaving no active transaction — does not shadow
/// the original error) and the error propagates.
pub fn write_txn<T, F>(conn: &Connection, body: F) -> rusqlite::Result<T>
where
    F: FnOnce(&Connection) -> rusqlite::Result<T>,
{
    conn.execute_batch("BEGIN IMMEDIATE")?;
    match body(conn) {
        Ok(value) => {
            conn.execute_batch("COMMIT")?;
            Ok(value)
        }
        Err(err) => {
            // Guarded rollback: swallow the OperationalError a
            // auto-rollback can produce, then re-raise the original.
            let _ = conn.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}
