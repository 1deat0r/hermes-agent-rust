//! Shared SQLite primitives for the small per-profile / board stores.
//!
//! PARITY: `hermes_cli/sqlite_util.py` @ 5d59366 (whole module).
//!
//! `open_db` is the one connect + PRAGMA stack; `transaction` is the one
//! commit-and-ALWAYS-close shape (#69567: a `with conn:` only commits, it
//! never closes — every hand-rolled pair leaked fds until GC). The WAL
//! half rides an injected `wal_setup` seam: hermes-state owns the real
//! `apply_wal_with_fallback` (reset-gate, network fallback,
//! never-live-downgrade); the default is a plain WAL pragma, documented.

use rusqlite::Connection;
use std::path::Path;

/// Options for [`open_db`], mirroring upstream's keyword args.
pub struct DbOptions {
    /// Store label for WAL diagnostics (upstream `db_label`, required).
    pub db_label: String,
    /// Busy timeout ms — passed as connect timeout AND explicit PRAGMA.
    pub busy_timeout_ms: i64,
    /// Journal mode via the WAL seam (upstream `wal=True`).
    pub wal: bool,
    /// `PRAGMA foreign_keys=ON`.
    pub foreign_keys: bool,
    /// `PRAGMA synchronous=FULL` (default NORMAL).
    pub synchronous_full: bool,
    /// Schema hook run after PRAGMAs (upstream `initialize`).
    pub initialize: Option<Box<dyn FnOnce(&Connection)>>,
    /// Lock-retry budget for transient "database is locked" during WAL
    /// setup (upstream `wal_lock_retries=1`, 10ms exponential backoff).
    pub wal_lock_retries: u32,
}

impl Default for DbOptions {
    fn default() -> Self {
        Self {
            db_label: String::new(),
            busy_timeout_ms: 5000,
            wal: true,
            foreign_keys: false,
            synchronous_full: false,
            initialize: None,
            wal_lock_retries: 1,
        }
    }
}

// PARITY: `open_db` (upstream lines 18-69) — open (parents created),
// PRAGMA stack, `initialize`; the connection is closed if anything raises.
pub fn open_db(
    path: &Path,
    options: DbOptions,
) -> Result<Connection, String> {
    open_db_with(path, options, None)
}

/// [`open_db`] with an injectable WAL-setup seam.
///
/// PARITY: the `apply_wal_with_fallback(conn, db_label=db_label)` call
/// (upstream line ~52) with its transient-locked retry loop. `wal_setup`
/// receives `(conn, db_label)`; `None` applies a plain `PRAGMA
/// journal_mode=WAL` (documented degradation — bypasses the reset-gate /
/// network-fallback / never-live-downgrade invariants hermes-state owns).
/// Only transient "database is locked" is retried; anything else raises.
pub fn open_db_with(
    path: &Path,
    options: DbOptions,
    wal_setup: Option<&dyn Fn(&Connection, &str) -> Result<(), String>>,
) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // `except BaseException: conn.close(); raise` — the slot reclaims the
    // connection on Err AND on panic (a panicking hook must not leak the fd
    // either); success takes it back out for the caller.
    let mut slot = Some(Connection::open(path).map_err(|e| e.to_string())?);
    struct Closer<'s> {
        slot: &'s mut Option<Connection>,
    }
    impl Drop for Closer<'_> {
        fn drop(&mut self) {
            if let Some(conn) = self.slot.take() {
                let _ = conn.close();
            }
        }
    }
    let result = (|| -> Result<(), String> {
        let guard = Closer { slot: &mut slot };
        let conn = guard.slot.as_ref().expect("slot holds conn during setup");
        conn.execute_batch(&format!("PRAGMA busy_timeout={}", options.busy_timeout_ms))
            .map_err(|e| e.to_string())?;
        // `connect(timeout=)` equivalent: rusqlite has no per-open timeout
        // below the busy-timeout PRAGMA, which is the observable knob.
        if options.wal {
            let mut attempts = options.wal_lock_retries.max(1);
            loop {
                let result = match wal_setup {
                    Some(setup) => setup(conn, &options.db_label),
                    None => conn
                        .execute_batch("PRAGMA journal_mode=WAL")
                        .map(|_| ())
                        .map_err(|e| e.to_string()),
                };
                match result {
                    Ok(()) => break,
                    Err(e)
                        if e.to_lowercase() == "database is locked" && attempts > 1 =>
                    {
                        attempts -= 1;
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        if options.foreign_keys {
            conn.execute_batch("PRAGMA foreign_keys=ON")
                .map_err(|e| e.to_string())?;
        }
        if options.synchronous_full {
            conn.execute_batch("PRAGMA synchronous=FULL")
                .map_err(|e| e.to_string())?;
        }
        if let Some(initialize) = options.initialize {
            initialize(conn);
        }
        // Success: disarm — the connection returns to the caller.
        std::mem::forget(guard);
        Ok(())
    })();
    match result {
        Ok(()) => Ok(slot.take().expect("slot holds conn after success")),
        Err(e) => Err(e),
    }
}

// PARITY: `transaction` (upstream lines 71-79) — commit on success, roll
/// back on error, and ALWAYS close `conn` (#69567). Takes ownership: the
/// caller cannot use the connection afterwards (compile-enforced close).
pub fn transaction<T>(
    conn: Connection,
    immediate: bool,
    body: impl FnOnce(&Connection) -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    // Upstream `with conn:` opens a DEFERRED transaction implicitly;
    // rusqlite needs the explicit BEGIN (DEFERRED default, IMMEDIATE opt).
    conn.execute_batch(if immediate {
        "BEGIN IMMEDIATE"
    } else {
        "BEGIN"
    })?;
    let result = (|| {
        let value = body(&conn)?;
        conn.execute_batch("COMMIT")?;
        Ok(value)
    })();
    if result.is_err() {
        let _ = conn.execute_batch("ROLLBACK");
    }
    let close = conn.close();
    match (result, close) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), _) => Err(e),
        (Ok(_), Err((_, e))) => Err(e),
    }
}

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
