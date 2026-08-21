//! hermes_state.py — SessionDB SQLite store (core lifecycle, connection).
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   module constants / default path            (87–267)
//!   live-DB test-isolation guard               (313–440)
//!   zeroed-DB quarantine                       (1939–2091)
//!   SessionDB.__init__ / open                  (2207–2458 read-only path, writable 2460+)
//!   _execute_write + retry sleep               (2768–3020)
//!   close                                      (3021–3124)
//!   get_meta / set_meta                        (9078–9121)
//!   _store_system_prompt                       (2178–2205)
//!   fts5_cjk extension helpers                 (1833–1872)

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::common::FTS_CJK_STALE_KEY;
use crate::schema;

/// sha256 hexdigest (moved to schema.rs); re-export for callers.
pub use crate::schema::system_prompt_hash;

// ── defaults ────────────────────────────────────────────────────────────────

fn default_db_path() -> PathBuf {
    hermes_constants::get_hermes_home().join("state.db")
}

// ── live-DB test-isolation guard (env-gated, mirrors upstream) ──────────────

fn running_under_pytest() -> bool {
    std::env::var("PYTEST_CURRENT_TEST").is_ok() || std::env::var("PYTEST_VERSION").is_ok()
}

fn real_platform_state_root() -> Option<PathBuf> {
    // os.path.expanduser("~") — reads HOME, not $PWD-dependent Path::home.
    if cfg!(windows) {
        if let Ok(base) = std::env::var("LOCALAPPDATA") {
            if !base.trim().is_empty() {
                return Some(PathBuf::from(base).join("hermes"));
            }
        }
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        return Some(PathBuf::from(home).join("AppData").join("Local").join("hermes"));
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Some(PathBuf::from(home).join(".hermes"))
}

fn is_production_state_db(resolved: &Path, root: &Path) -> bool {
    if resolved.parent() == Some(root) {
        return true;
    }
    let Ok(rel) = resolved.strip_prefix(root) else {
        return false;
    };
    let parts: Vec<_> = rel.components().collect();
    parts.len() == 3 && parts[0].as_os_str() == "profiles"
}

fn ensure_test_isolation(db_path: &Path) -> Result<(), String> {
    if !running_under_pytest() {
        return Ok(());
    }
    let Ok(resolved) = db_path.canonicalize().or_else(|_| {
        db_path
            .parent()
            .map(|p| p.canonicalize().map(|c| c.join(db_path.file_name().unwrap_or_default())))
            .unwrap_or_else(|| std::fs::canonicalize(db_path))
    }) else {
        return Ok(());
    };
    if let Some(root) = real_platform_state_root() {
        if is_production_state_db(&resolved, &root) {
            return Err(format!(
                "live-system guard: test attempted to open production state.db at {} (under real Hermes root {}). Tests must run against a temporary HERMES_HOME — pass an explicit tmp db_path or let the hermetic conftest redirect HERMES_HOME. If this test genuinely needs the live database, mark it with @pytest.mark.live_system_guard_bypass.",
                resolved.display(),
                root.display()
            ));
        }
    }
    Ok(())
}

// ── zeroed-DB detection / quarantine (#68474 / #68805) ──────────────────────

/// Detect the zeroed state.db signature (size>0, NUL header).
// PARITY: hermes_state.py is_zeroed_state_db @ b9aa928 (local fallback form)
pub fn is_zeroed_state_db(path: &Path) -> bool {
    let Ok(md) = std::fs::metadata(path) else {
        return false;
    };
    if md.len() == 0 {
        return false;
    }
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    use std::io::Read;
    let probe_len = 100usize;
    let mut head = vec![0u8; probe_len];
    let n = f.read(&mut head).unwrap_or(0);
    if n == 0 {
        return false;
    }
    head.truncate(n);
    if head.starts_with(b"SQLite format 3") {
        return false;
    }
    head.iter().all(|b| *b == 0)
}

/// Move a zeroed state.db aside (preserve bytes) and return quarantine path.
///
/// Uses a cross-process lock file so two concurrent startups cannot race.
// PARITY: hermes_state.py quarantine_zeroed_state_db @ b9aa928
pub fn quarantine_zeroed_state_db(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let _ = std::fs::create_dir_all(dir);
    let lock_path = dir.join(format!("{}.quarantine.lock", path.file_name()?.to_string_lossy()));
    let lock = std::fs::File::options()
        .create(true)
        .append(true)
        .open(&lock_path)
        .ok()?;
    // POSIX advisory exclusive lock (flock). Windows falls back to best-effort
    // without the lock (documented divergence; POSIX is the port target).
    let locked = {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let rc = unsafe { libc_flock(lock.as_raw_fd()) };
            rc == 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    };
    let _ = &lock; // keep handle alive
    if !locked {
        return None;
    }
    // Re-check under the lock (a concurrent process may have already moved it).
    if !is_zeroed_state_db(path) {
        return None;
    }
    let quarantine = dir.join("state.db.zeroed-quarantine");
    // Unique name if a previous quarantine exists.
    let quarantine = if quarantine.exists() {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        dir.join(format!("state.db.zeroed-quarantine.{}", n))
    } else {
        quarantine
    };
    std::fs::rename(path, &quarantine).ok()?;
    Some(quarantine)
}

#[cfg(unix)]
unsafe fn libc_flock(fd: i32) -> i32 {
    extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 2;
    flock(fd, LOCK_EX)
}

// ── CJK extension helpers ───────────────────────────────────────────────────

/// Location of the cjk_unicode61 loadable extension.
// PARITY: hermes_state.py fts5_cjk_so_path @ b9aa928
pub fn fts5_cjk_so_path() -> PathBuf {
    if let Ok(env) = std::env::var("HERMES_FTS5_CJK_SO") {
        let p = PathBuf::from(env);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    hermes_constants::get_hermes_home().join("lib").join("libfts5_cjk.so")
}

/// config.yaml `sessions.cjk_fts` (default on), via its env bridge.
// PARITY: hermes_state.py _cjk_fts_config_enabled @ b9aa928
pub fn cjk_fts_config_enabled() -> bool {
    let raw = std::env::var("HERMES_CJK_FTS").unwrap_or_else(|_| "1".to_string());
    !matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no"
    )
}

/// Best-effort load of the cjk_unicode61 tokenizer (does not raise).
// PARITY: hermes_state.py load_fts5_cjk_extension @ b9aa928
pub fn load_fts5_cjk_extension(conn: &Connection) -> bool {
    if !cjk_fts_config_enabled() {
        return false;
    }
    let path = fts5_cjk_so_path();
    if !path.exists() {
        return false;
    }
    unsafe {
        match conn.load_extension(&path, None::<&str>) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("fts5_cjk extension load failed ({}): {}", path.display(), e);
                false
            }
        }
    }
}

// ── SessionDB ───────────────────────────────────────────────────────────────

/// Last `SessionDB` init error, per-process (surfaced by callers' /resume).
static LAST_INIT_ERROR: Lazy<std::sync::Mutex<Option<String>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

pub fn set_last_init_error(msg: Option<String>) {
    *LAST_INIT_ERROR.lock().unwrap() = msg;
}

pub fn get_last_init_error() -> Option<String> {
    LAST_INIT_ERROR.lock().unwrap().clone()
}

/// SQLite-backed session storage (SessionDB). Thread-affine for the writer
/// connection in this build (the cross-thread read-path split ships with the
/// full CRUD port); schema/init/meta surfaces are complete.
pub struct SessionDB {
    pub db_path: PathBuf,
    pub read_only: bool,
    conn: RefCell<Option<Connection>>,
    // interior-mutable capability flags (upstream plain attributes)
    fts_enabled: Cell<bool>,
    trigram_available: Cell<bool>,
    fts_cjk_loaded: Cell<bool>,
    fts_cjk_available: Cell<bool>,
    fts_unavailable_warned: Cell<bool>,
    trigram_unavailable_warned: Cell<bool>,
    wal_active: Cell<bool>,
    write_count: Cell<u64>,
    fts_runtime_rebuild_attempted: Cell<bool>,
    fts_usermerge_floor_applied: Cell<bool>,
}

/// Write-contention tuning — constants mirroring the class attributes.
impl SessionDB {
    pub const WRITE_PATIENCE_S: f64 = 20.0;
    pub const TRANSCRIPT_WRITE_PATIENCE_S: f64 = 60.0;
    pub const ACTIVITY_WRITE_PATIENCE_S: f64 = 0.5;
    pub const WRITE_RETRY_MIN_S: f64 = 0.020;
    pub const WRITE_RETRY_MAX_S: f64 = 0.150;
    pub const WRITE_RETRY_SLOW_AFTER_S: f64 = 2.0;
    pub const WRITE_RETRY_SLOW_MIN_S: f64 = 0.250;
    pub const WRITE_RETRY_SLOW_MAX_S: f64 = 1.000;
    pub const CHECKPOINT_EVERY_N_WRITES: u64 = 50;
    pub const FTS_MERGE_EVERY_N_WRITES: u64 = 1000;
    pub const FTS_TRASH_PREFIX: &'static str = "fts_v22_trash_";
}

impl SessionDB {
    /// Open a SessionDB (writable or read-only).
    ///
    /// PARITY: SessionDB.__init__ @ b9aa928 (2207–2458). Deferred to the
    /// full-CRUD port: writable preflight (repair-or-refuse), malformed-
    /// schema in-place repair, sqlite_safe_read tracked connections.
    pub fn open(db_path: Option<PathBuf>, read_only: bool) -> Result<SessionDB, String> {
        let db_path = db_path.unwrap_or_else(default_db_path);
        ensure_test_isolation(&db_path)?;

        let mut db = SessionDB {
            db_path,
            read_only,
            conn: RefCell::new(None),
            fts_enabled: Cell::new(false),
            trigram_available: Cell::new(false),
            fts_cjk_loaded: Cell::new(false),
            fts_cjk_available: Cell::new(false),
            fts_unavailable_warned: Cell::new(false),
            trigram_unavailable_warned: Cell::new(false),
            wal_active: Cell::new(false),
            write_count: Cell::new(0),
            fts_runtime_rebuild_attempted: Cell::new(false),
            fts_usermerge_floor_applied: Cell::new(false),
        };

        match db.open_inner(read_only) {
            Ok(()) => Ok(db),
            Err(msg) => {
                set_last_init_error(Some(msg.clone()));
                Err(msg)
            }
        }
    }

    fn open_inner(&mut self, read_only: bool) -> Result<(), String> {
        if read_only {
            return self.open_read_only();
        }
        self.open_writable()
    }

    fn open_read_only(&mut self) -> Result<(), String> {
        // URI read-only attach; no schema init (SELECT-only), no write lock.
        let uri = format!("file:{}?mode=ro", self.db_path.display());
        let conn = Connection::open_with_flags(&uri, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)
            .map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(1000))
            .map_err(|e| e.to_string())?;
        // Read-only opens must not touch the FTS probe if schema is malformed;
        // on ANY probe failure close the connection and propagate.
        #[allow(clippy::redundant_closure_call)]
        let probe_result = (|| -> Result<(), rusqlite::Error> {
            crate::wal::apply_database_pragmas(&conn, "state.db");
            let has_base = schema::fts_table_probe(&conn, "messages_fts") == Some(true);
            self.fts_enabled.set(has_base);
            if has_base {
                let has_trigram =
                    schema::fts_table_probe(&conn, "messages_fts_trigram") == Some(true);
                self.trigram_available.set(has_trigram);
            }
            Ok(())
        })();
        match probe_result {
            Ok(()) => {
                *self.conn.borrow_mut() = Some(conn);
                Ok(())
            }
            Err(e) => {
                // Close the connection on the failure path (upstream leaks
                // protection: a leaked tracked connection blocks the backup
                // raw-copy).
                drop(conn);
                self.conn.borrow_mut().take();
                Err(format!("{}", e))
            }
        }
    }

    fn open_writable(&mut self) -> Result<(), String> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // #68474 zeroed state.db handling.
        if self.db_path.exists() && is_zeroed_state_db(&self.db_path) {
            let zsize = std::fs::metadata(&self.db_path).map(|m| m.len() as i64).unwrap_or(-1);
            let snaps = self.db_path.parent().unwrap_or(Path::new(".")).join("state-snapshots");
            let qpath = quarantine_zeroed_state_db(&self.db_path);
            let msg = format!(
                "state.db looks ZEROED ({} bytes, no SQLite header). Preserved at {}. Restore from {} via `hermes snapshot list` / `hermes snapshot restore <id>` if available. Opening a fresh empty database so the agent can start.",
                zsize,
                qpath.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(quarantine failed — file left in place)".to_string()),
                snaps.display(),
            );
            eprintln!("{}", msg);
            set_last_init_error(Some(msg));
            if qpath.is_none() && self.db_path.exists() && is_zeroed_state_db(&self.db_path) {
                return Err(format!(
                    "state.db looks ZEROED ({} bytes, no SQLite header) and quarantine failed — refusing to open the damaged file",
                    zsize
                ));
            }
        }

        // Connect + init with lock patience (mirrors
        // _connect_and_init_with_lock_patience).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(Self::WRITE_PATIENCE_S);
        loop {
            match self.connect_and_init() {
                Ok(()) => return Ok(()),
                Err(msg) => {
                    let lowered = msg.to_ascii_lowercase();
                    if !lowered.contains("locked") && !lowered.contains("busy") {
                        return Err(msg);
                    }
                    self.conn.borrow_mut().take();
                    if std::time::Instant::now() >= deadline {
                        return Err(msg);
                    }
                    let elapsed = deadline.elapsed().as_secs_f64();
                    let patience = Self::WRITE_PATIENCE_S;
                    let slow = elapsed >= Self::WRITE_RETRY_SLOW_AFTER_S;
                    let (lo, hi) = if slow {
                        (Self::WRITE_RETRY_SLOW_MIN_S, Self::WRITE_RETRY_SLOW_MAX_S)
                    } else {
                        (Self::WRITE_RETRY_MIN_S, Self::WRITE_RETRY_MAX_S)
                    };
                    let jitter = lo + rand_uniform() * (hi - lo);
                    let sleep_for = jitter.min((deadline - std::time::Instant::now()).as_secs_f64().max(0.001));
                    std::thread::sleep(std::time::Duration::from_secs_f64(sleep_for));
                    let _ = patience;
                }
            }
        }
    }

    fn connect_and_init(&self) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(1000))
            .map_err(|e| e.to_string())?;
        let wal_mode = crate::wal::apply_wal_with_fallback(&conn, "state.db", false)
            .map_err(|e| e.0)?;
        self.wal_active.set(wal_mode == "wal");
        crate::wal::apply_database_pragmas(&conn, "state.db");
        conn.execute_batch("PRAGMA foreign_keys=ON").map_err(|e| e.to_string())?;
        let cjk_loaded = load_fts5_cjk_extension(&conn);
        self.fts_cjk_loaded.set(cjk_loaded);
        // The connection must be visible before schema init: the schema
        // mixin's migrate_broad_fts_update_triggers reaches the host
        // connection through writer_conn(), exactly as the Python mixin
        // reaches `self._conn`.
        *self.conn.borrow_mut() = Some(conn);
        // schema init lives in schema.rs (impl below via init_schema).
        let result = self.init_schema_inner(&self.writer_conn());
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                // Close the failed connection so callers can retry cleanly.
                self.conn.borrow_mut().take();
                Err(e)
            }
        }
    }

    /// Borrow the writer connection (panics after close — mirrors upstream's
    /// RuntimeError on use-after-close).
    pub fn writer_conn(&self) -> std::cell::Ref<'_, Connection> {
        std::cell::Ref::map(self.conn.borrow(), |c| {
            c.as_ref().expect("SessionDB connection is closed")
        })
    }

    pub fn close(&self) {
        {
            let conn_opt = self.conn.borrow();
            if let Some(conn) = conn_opt.as_ref() {
                if !self.read_only {
                    // Writables attempt a TRUNCATE WAL checkpoint.
                    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
                }
            }
        }
        self.conn.borrow_mut().take();
    }

    /// Execute a write transaction with BEGIN IMMEDIATE and jitter retry.
    ///
    /// PARITY: hermes_state.py _execute_write @ b9aa928 (2768–2897) minus the
    /// compression-lock and runtime-FTS-rebuild paths (those arrive with the
    /// compression + search ports; the retry machinery is identical).
    pub fn execute_write<F, T>(&self, f: &F, patience_s: Option<f64>) -> Result<T, String>
    where
        F: Fn(&Connection) -> Result<T, rusqlite::Error>,
    {
        let patience_s = patience_s.unwrap_or(Self::WRITE_PATIENCE_S);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(patience_s);
        loop {
            let conn = self.writer_conn();
            let attempt = (|| -> Result<T, rusqlite::Error> {
                conn.execute_batch("BEGIN IMMEDIATE")?;
                let result = f(&conn);
                match result {
                    Ok(v) => {
                        conn.execute_batch("COMMIT")?;
                        Ok(v)
                    }
                    Err(e) => {
                        let _ = conn.execute_batch("ROLLBACK");
                        Err(e)
                    }
                }
            })();
            match attempt {
                Ok(v) => {
                    let count = self.write_count.get() + 1;
                    self.write_count.set(count);
                    if count.is_multiple_of(Self::CHECKPOINT_EVERY_N_WRITES) {
                        self.try_wal_checkpoint();
                    }
                    if count.is_multiple_of(Self::FTS_MERGE_EVERY_N_WRITES) {
                        let _ = self.try_incremental_merge_fts();
                    }
                    return Ok(v);
                }
                Err(rusqlite::Error::SqliteFailure(e, _)) => {
                    let msg = e.to_string().to_ascii_lowercase();
                    if msg.contains("locked") || msg.contains("busy") {
                        if !self.sleep_before_write_retry(deadline, patience_s) {
                            return Err(format!(
                                "database is locked (another Hermes process held the state.db write lock for over {:.0}s — likely a long maintenance operation such as VACUUM, a large WAL checkpoint, or an older pre-update process; the database itself is healthy)",
                                patience_s
                            ));
                        }
                        continue;
                    }
                    if msg.contains("no more rows available")
                        && self.sleep_before_write_retry(deadline, patience_s)
                    {
                        continue;
                    }
                    return Err(e.to_string());
                }
                Err(e) => {
                    let msg = e.to_string().to_ascii_lowercase();
                    if msg.contains("no more rows available") && self.sleep_before_write_retry(deadline, patience_s) {
                        continue;
                    }
                    return Err(e.to_string());
                }
            }
        }
    }

    /// Sleep one jitter interval if the patience budget still allows it.
    // PARITY: hermes_state.py _sleep_before_write_retry @ b9aa928 (2898–2927)
    pub fn sleep_before_write_retry(
        &self,
        deadline: std::time::Instant,
        patience_s: f64,
    ) -> bool {
        let now = std::time::Instant::now();
        if now >= deadline {
            return false;
        }
        let elapsed = now.duration_since(deadline - std::time::Duration::from_secs_f64(patience_s))
            .as_secs_f64();
        let (jitter_lo, jitter_hi) = if elapsed >= Self::WRITE_RETRY_SLOW_AFTER_S {
            (Self::WRITE_RETRY_SLOW_MIN_S, Self::WRITE_RETRY_SLOW_MAX_S)
        } else {
            (Self::WRITE_RETRY_MIN_S, Self::WRITE_RETRY_MAX_S)
        };
        let jitter = jitter_lo + rand_uniform() * (jitter_hi - jitter_lo);
        let sleep_for = jitter.min((deadline - now).as_secs_f64().max(0.001));
        std::thread::sleep(std::time::Duration::from_secs_f64(sleep_for));
        true
    }

    /// Best-effort PASSIVE WAL checkpoint. Never raises.
    // PARITY: hermes_state.py _try_wal_checkpoint @ b9aa928 (2991–3020)
    pub fn try_wal_checkpoint(&self) {
        let conn = self.writer_conn();
        let _ = conn.query_row(
            "PRAGMA wal_checkpoint(PASSIVE)",
            [],
            |r| Ok((r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
        );
    }

    /// Hook for the FTS bounded-merge cadence (search port provides the real
    /// implementation; no-op until then — mirrors a bare bones default).
    pub fn try_incremental_merge_fts(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Option<String> {
        let conn = self.writer_conn();
        conn.query_row(
            "SELECT value FROM state_meta WHERE key = ?",
            [key],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), String> {
        let op = |conn: &Connection| -> Result<(), rusqlite::Error> {
            conn.execute(
                "INSERT INTO state_meta (key, value) VALUES (?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        };
        self.execute_write(&op, None)
    }

    // Accessors used by the schema mixin -------------------------------------------------

    pub fn fts_enabled(&self) -> bool { self.fts_enabled.get() }
    pub fn set_fts_enabled(&self, v: bool) { self.fts_enabled.set(v); }
    pub fn trigram_available(&self) -> bool { self.trigram_available.get() }
    pub fn set_trigram_available(&self, v: bool) { self.trigram_available.set(v); }
    pub fn fts_cjk_loaded(&self) -> bool { self.fts_cjk_loaded.get() }
    pub fn set_fts_cjk_loaded(&self, v: bool) { self.fts_cjk_loaded.set(v); }
    pub fn fts_cjk_available(&self) -> bool { self.fts_cjk_available.get() }
    pub fn set_fts_cjk_available(&self, v: bool) { self.fts_cjk_available.set(v); }
    pub fn fts_unavailable_warned(&self) -> bool { self.fts_unavailable_warned.get() }
    pub fn set_fts_unavailable_warned(&self, v: bool) { self.fts_unavailable_warned.set(v); }
    pub fn trigram_unavailable_warned(&self) -> bool { self.trigram_unavailable_warned.get() }
    pub fn set_trigram_unavailable_warned(&self, v: bool) { self.trigram_unavailable_warned.set(v); }
    pub fn wal_active(&self) -> bool { self.wal_active.get() }
    pub fn set_wal_active(&self, v: bool) { self.wal_active.set(v); }
    pub fn write_count(&self) -> u64 { self.write_count.get() }
    pub fn fts_runtime_rebuild_attempted(&self) -> bool { self.fts_runtime_rebuild_attempted.get() }
    pub fn set_fts_runtime_rebuild_attempted(&self, v: bool) { self.fts_runtime_rebuild_attempted.set(v); }
    pub fn fts_usermerge_floor_applied(&self) -> bool { self.fts_usermerge_floor_applied.get() }
    pub fn set_fts_usermerge_floor_applied(&self, v: bool) { self.fts_usermerge_floor_applied.set(v); }

    pub fn cjk_update_trigger_is_narrowed(&self, conn: &Connection) -> bool {
        schema::cjk_update_trigger_is_narrowed(conn)
    }

    pub fn quarantine_cjk_after_update_of_migration(&self, conn: &Connection) {
        // Fail-closed after dropping CJK UPDATE during OF migration.
        self.fts_cjk_available.set(false);
        let _ = self.set_meta_cursor(conn, FTS_CJK_STALE_KEY, "1");
        let _ = conn.execute_batch("DROP TRIGGER IF EXISTS messages_fts_cjk_update");
    }

    pub(crate) fn set_meta_cursor(
        &self,
        conn: &Connection,
        key: &str,
        value: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO state_meta (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}

/// Deterministic-ish uniform(0,1) without pulling in a rand dependency
/// (Xorshift64* — perfectly adequate for jitter).
fn rand_uniform() -> f64 {
    use std::cell::Cell;
    thread_local! {
        static SEED: Cell<u64> = const { Cell::new(0x9E3779B97F4A7C15) };
    }
    SEED.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}
