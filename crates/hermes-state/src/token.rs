//! Async token accounting — SessionDB background writer queue.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   queue_token_counts / flush_token_counts / _token_writer_loop /
//!   _apply_token_batch / _coalesce_token_deltas / _stop_token_writer /
//!   _drain_token_queue_at_exit   (4913–5143)
//!   update_token_counts          (5143–5331)
//!   _record_model_usage          (5333–5424)
//!   ensure_session               (5425–5444)
//!   record_auxiliary_usage       (5445–5483)
//!
//! Divergence (documented): upstream shares ONE sqlite connection under
//! `self._lock`; a background thread calling `update_token_counts` would
//! need `&SessionDB` across threads and SessionDB is `!Sync` in Rust. The
//! port therefore gives the writer thread its own dedicated read-write
//! connection, opened at spawn and closed on thread exit. Observable
//! semantics are preserved:
//!   - deltas apply strictly in enqueue order from the single writer thread;
//!   - every apply runs as its own BEGIN IMMEDIATE write transaction with
//!     the same jitter/busy-retry budget as the SessionDB write path;
//!   - flush / stop / close follow the same busy-before-clear protocol;
//!   - synchronous fallbacks (writer stopped or dead) use the SessionDB's
//!     own connection, exactly like the inline paths.
//!
//! The writer connection skips the sessions-DB checkpoint cadence
//! (write_count / WAL checkpoint / FTS merge hooks). That cadence is a
//! performance optimization only: token applies are small and comparatively
//! infrequent.
//!
//! There is no atexit equivalent in Rust; `SessionDB::close()` joins the
//! writer (bounded) and drains leftovers synchronously, and a `Drop` safety
//! net leaks the shared queue state if a SessionDB is dropped while the
//! writer is still alive (daemon-equivalent to upstream's atexit drain).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension};

use crate::crud::{insert_session_row_on, NewSession};
use crate::state::{now, SessionDB, WriteError};

// ── coalescing field classification (mirrors upstream class attrs) ──────────

/// Delta fields whose values SUM when two same-route deltas merge.
pub const TOKEN_DELTA_SUM_FIELDS: [&str; 6] = [
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_tokens",
    "api_call_count",
];
/// Delta fields summed only when both sides carry a value (None-preserving).
pub const TOKEN_DELTA_COST_FIELDS: [&str; 2] = ["estimated_cost_usd", "actual_cost_usd"];
/// Delta fields that must be EQUAL for two deltas to merge (the route).
pub const TOKEN_DELTA_ROUTE_FIELDS: [&str; 7] = [
    "model",
    "cost_status",
    "cost_source",
    "pricing_version",
    "billing_provider",
    "billing_base_url",
    "billing_mode",
];

// ── delta type ──────────────────────────────────────────────────────────────

/// Keyword-style arguments for `update_token_counts` (mirrors upstream's
/// full kwargs surface; `_TOKEN_DELTA_*` classify every field).
#[derive(Debug, Clone, Default)]
pub struct TokenDelta {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub reasoning_tokens: i64,
    pub estimated_cost_usd: Option<f64>,
    pub actual_cost_usd: Option<f64>,
    pub cost_status: Option<String>,
    pub cost_source: Option<String>,
    pub pricing_version: Option<String>,
    pub billing_provider: Option<String>,
    pub billing_base_url: Option<String>,
    pub billing_mode: Option<String>,
    pub api_call_count: i64,
    /// absolute=True sets cumulative totals directly (never merges).
    pub absolute: bool,
}

impl TokenDelta {
    pub fn has_accounted_usage(&self) -> bool {
        self.input_tokens != 0
            || self.output_tokens != 0
            || self.cache_read_tokens != 0
            || self.cache_write_tokens != 0
            || self.reasoning_tokens != 0
            || self.api_call_count != 0
            || self.estimated_cost_usd.is_some()
            || self.actual_cost_usd.is_some()
    }

    pub fn record_model_usage(&self) -> bool {
        !self.absolute
            && (self.input_tokens != 0
                || self.output_tokens != 0
                || self.cache_read_tokens != 0
                || self.cache_write_tokens != 0
                || self.reasoning_tokens != 0
                || self.api_call_count != 0
                || self.estimated_cost_usd.is_some())
    }
}

// ── shared queue state ──────────────────────────────────────────────────────

#[derive(Default)]
struct TokenQueueInner {
    queue: VecDeque<(String, TokenDelta)>,
    stop: bool,
    busy: bool,
}

/// Shared Mutex+Condvar queue, owned by the SessionDB and cloned into the
/// writer thread (and any stop/join helper). The `dead` flag is set when a
/// writer thread exits without a stop request (unexpected escape / spawn
/// failure), matching upstream's `not thread.is_alive()` respawn logic.
pub(crate) struct TokenWriterShared {
    inner: Mutex<TokenQueueInner>,
    cond: Condvar,
    dead: AtomicBool,
}

impl TokenWriterShared {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(TokenWriterShared {
            inner: Mutex::new(TokenQueueInner::default()),
            cond: Condvar::new(),
            dead: AtomicBool::new(true),
        })
    }
}

fn lock_inner(shared: &TokenWriterShared) -> MutexGuard<'_, TokenQueueInner> {
    shared.inner.lock().unwrap_or_else(PoisonError::into_inner)
}

fn lock_thread(db: &SessionDB) -> MutexGuard<'_, Option<std::thread::JoinHandle<()>>> {
    db.token_writer_thread
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

// ── writer connection (dedicated) ───────────────────────────────────────────

fn open_writer_conn(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_millis(1000))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn log_warn(msg: &str) {
    eprintln!("[hermes-state] WARN: {}", msg);
}

// ── write transaction helper (mirrors SessionDB::execute_write loop) ────────

/// Run `f` inside BEGIN IMMEDIATE with the standard jitter/busy retry loop.
/// Used by the token writer's dedicated connection; no checkpoint/FTS
/// hooks (documented divergence — those are per-session-DB-bookkeeping).
fn execute_write_on<F, T>(conn: &Connection, f: &F) -> Result<T, WriteError>
where
    F: Fn(&Connection) -> Result<T, WriteError>,
{
    let patience_s = SessionDB::WRITE_PATIENCE_S;
    let deadline = Instant::now() + Duration::from_secs_f64(patience_s);
    loop {
        let attempt = (|| -> Result<T, WriteError> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            let result = f(conn);
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
            Ok(v) => return Ok(v),
            Err(WriteError::Sqlite(rusqlite::Error::SqliteFailure(e, _))) => {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("locked") || msg.contains("busy") {
                    if !sleep_before_write_retry(deadline, patience_s) {
                        return Err(WriteError::Sqlite(rusqlite::Error::SqliteFailure(e, None)));
                    }
                    continue;
                }
                if msg.contains("no more rows available")
                    && sleep_before_write_retry(deadline, patience_s)
                {
                    continue;
                }
                return Err(WriteError::Sqlite(rusqlite::Error::SqliteFailure(e, None)));
            }
            Err(WriteError::Sqlite(e)) => {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("no more rows available")
                    && sleep_before_write_retry(deadline, patience_s)
                {
                    continue;
                }
                return Err(WriteError::Sqlite(e));
            }
            Err(e) => return Err(e),
        }
    }
}

fn sleep_before_write_retry(deadline: Instant, _patience_s: f64) -> bool {
    let now = Instant::now();
    if now >= deadline {
        return false;
    }
    let elapsed = deadline.duration_since(now).as_secs_f64();
    let slow = elapsed >= SessionDB::WRITE_RETRY_SLOW_AFTER_S;
    let (lo, hi) = if slow {
        (
            SessionDB::WRITE_RETRY_SLOW_MIN_S,
            SessionDB::WRITE_RETRY_SLOW_MAX_S,
        )
    } else {
        (SessionDB::WRITE_RETRY_MIN_S, SessionDB::WRITE_RETRY_MAX_S)
    };
    let jitter = lo + rand_uniform() * (hi - lo);
    let budget = elapsed.max(0.001);
    let sleep_for = jitter.min(budget);
    std::thread::sleep(Duration::from_secs_f64(sleep_for));
    true
}

fn rand_uniform() -> f64 {
    // xorshift64* — cheap thread-independent RNG (mirrors the existing
    // jitter helper's distribution).
    use std::cell::Cell;
    thread_local! {
        static SEED: Cell<u64> = const { Cell::new(0x9E3779B97F4A7C15) };
    }
    SEED.with(|s| {
        let mut x = s.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        s.set(x);
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    })
}

// ── update_token_counts on a connection ─────────────────────────────────────

const UPDATE_ABSOLUTE_SQL: &str = "UPDATE sessions SET
                   input_tokens = ?,
                   output_tokens = ?,
                   cache_read_tokens = ?,
                   cache_write_tokens = ?,
                   reasoning_tokens = ?,
                   estimated_cost_usd = COALESCE(?, 0),
                   actual_cost_usd = CASE
                       WHEN ? IS NULL THEN actual_cost_usd
                       ELSE ?
                   END,
                   cost_status = COALESCE(?, cost_status),
                   cost_source = COALESCE(?, cost_source),
                   pricing_version = COALESCE(?, pricing_version),
                   billing_provider = COALESCE(billing_provider, ?),
                   billing_base_url = COALESCE(billing_base_url, ?),
                   billing_mode = COALESCE(billing_mode, ?),
                   model = COALESCE(model, ?),
                   api_call_count = ?
                   WHERE id = ?";

const UPDATE_INCREMENTAL_SQL: &str = "UPDATE sessions SET
                   input_tokens = input_tokens + ?,
                   output_tokens = output_tokens + ?,
                   cache_read_tokens = cache_read_tokens + ?,
                   cache_write_tokens = cache_write_tokens + ?,
                   reasoning_tokens = reasoning_tokens + ?,
                   estimated_cost_usd = COALESCE(estimated_cost_usd, 0) + COALESCE(?, 0),
                   actual_cost_usd = CASE
                       WHEN ? IS NULL THEN actual_cost_usd
                       ELSE COALESCE(actual_cost_usd, 0) + ?
                   END,
                   cost_status = COALESCE(?, cost_status),
                   cost_source = COALESCE(?, cost_source),
                   pricing_version = COALESCE(?, pricing_version),
                   billing_provider = COALESCE(billing_provider, ?),
                   billing_base_url = COALESCE(billing_base_url, ?),
                   billing_mode = COALESCE(billing_mode, ?),
                   model = COALESCE(model, ?),
                   api_call_count = COALESCE(api_call_count, 0) + ?
                   WHERE id = ?";

/// Shared body of `SessionDB.update_token_counts` — run inside the caller's
/// write transaction. Includes the `first_accounted_route` pre-read and the
/// per-model usage attribution upsert.
///
/// PARITY: hermes_state.py update_token_counts._do @ b9aa928 (5236–5331)
fn apply_token_update_on(
    conn: &Connection,
    session_id: &str,
    delta: &TokenDelta,
) -> Result<(), WriteError> {
    let has_accounted_usage = delta.has_accounted_usage();

    // First-accounted-route pre-read (upstream's `_do` head): the row must
    // reflect the FIRST authoritative route before the summary UPDATE runs.
    let (existing_model, existing_provider, existing_api_calls) = conn
        .query_row(
            "SELECT model, billing_provider, api_call_count FROM sessions WHERE id = ?",
            rusqlite::params![session_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ))
            },
        )
        .optional()?
        .unwrap_or((None, None, 0));

    let first_accounted_route = existing_api_calls == 0
        && has_accounted_usage
        && delta.model.is_some()
        && delta.billing_provider.is_some()
        && (existing_model != delta.model || existing_provider != delta.billing_provider);
    if first_accounted_route {
        conn.execute(
            "UPDATE sessions
               SET model = ?, billing_provider = ?,
               billing_base_url = ?, billing_mode = ?
               WHERE id = ?",
            rusqlite::params![
                delta.model,
                delta.billing_provider,
                delta.billing_base_url,
                delta.billing_mode,
                session_id
            ],
        )?;
    }

    // Bind route fields to locals first: `route_or_none` returns a
    // temporary Option<&str> that must outlive the params borrow.
    let bp = route_or_none(delta.billing_provider.as_ref(), has_accounted_usage);
    let bu = route_or_none(delta.billing_base_url.as_ref(), has_accounted_usage);
    let bm = route_or_none(delta.billing_mode.as_ref(), has_accounted_usage);
    let mdl = route_or_none(delta.model.as_ref(), has_accounted_usage);
    let params: Vec<&dyn rusqlite::ToSql> = vec![
        &delta.input_tokens,
        &delta.output_tokens,
        &delta.cache_read_tokens,
        &delta.cache_write_tokens,
        &delta.reasoning_tokens,
        &delta.estimated_cost_usd,
        &delta.actual_cost_usd,
        &delta.actual_cost_usd,
        &delta.cost_status,
        &delta.cost_source,
        &delta.pricing_version,
        &bp,
        &bu,
        &bm,
        &mdl,
        &delta.api_call_count,
        &session_id,
    ];

    let sql = if delta.absolute {
        UPDATE_ABSOLUTE_SQL
    } else {
        UPDATE_INCREMENTAL_SQL
    };
    conn.execute(sql, rusqlite::params_from_iter(params))?;

    if delta.record_model_usage() {
        record_model_usage_on(conn, session_id, delta, "")?;
    }
    Ok(())
}

fn route_or_none(value: Option<&String>, has_accounted_usage: bool) -> Option<&str> {
    if has_accounted_usage {
        value.map(|s| s.as_str())
    } else {
        None
    }
}

/// Insert/upsert one per-model usage row. `task == ""` is the main agent
/// loop — rows inherit the session's route when the delta omits it;
/// aux-task rows (`task != ""`) never inherit (upstream issue #23270 /
/// the `_record_model_usage` eff-route rules).
///
/// PARITY: hermes_state.py _record_model_usage @ b9aa928 (5333–5424)
fn record_model_usage_on(
    conn: &Connection,
    session_id: &str,
    delta: &TokenDelta,
    task: &str,
) -> Result<(), WriteError> {
    insert_model_usage_on(
        conn,
        session_id,
        delta.model.as_deref(),
        delta.billing_provider.as_deref(),
        delta.billing_base_url.as_deref(),
        delta.billing_mode.as_deref(),
        task,
        delta.input_tokens,
        delta.output_tokens,
        delta.cache_read_tokens,
        delta.cache_write_tokens,
        delta.reasoning_tokens,
        delta.estimated_cost_usd,
        delta.actual_cost_usd,
        delta.cost_status.as_deref(),
        delta.cost_source.as_deref(),
        delta.api_call_count,
    )
}

/// The session_model_usage upsert (shared by the incremental update path
/// and `record_auxiliary_usage`).
///
/// PARITY: hermes_state.py _record_model_usage body @ b9aa928 (5355–5424)
#[allow(clippy::too_many_arguments)]
fn insert_model_usage_on(
    conn: &Connection,
    session_id: &str,
    model: Option<&str>,
    billing_provider: Option<&str>,
    billing_base_url: Option<&str>,
    billing_mode: Option<&str>,
    task: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    reasoning_tokens: i64,
    estimated_cost_usd: Option<f64>,
    actual_cost_usd: Option<f64>,
    cost_status: Option<&str>,
    cost_source: Option<&str>,
    api_call_count: i64,
) -> Result<(), WriteError> {
    let (sess_model, sess_provider, sess_base_url, sess_billing_mode) = conn
        .query_row(
            "SELECT model, billing_provider, billing_base_url, billing_mode \
             FROM sessions WHERE id = ?",
            rusqlite::params![session_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?
        .unwrap_or((None, None, None, None));

    let (eff_model, eff_provider, eff_base_url, eff_billing_mode) = if !task.is_empty() {
        (
            model.unwrap_or("unknown").to_string(),
            billing_provider.unwrap_or("").to_string(),
            billing_base_url.unwrap_or("").to_string(),
            billing_mode.unwrap_or("").to_string(),
        )
    } else {
        (
            model
                .or(sess_model.as_deref())
                .unwrap_or("unknown")
                .to_string(),
            billing_provider
                .or(sess_provider.as_deref())
                .unwrap_or("")
                .to_string(),
            billing_base_url
                .or(sess_base_url.as_deref())
                .unwrap_or("")
                .to_string(),
            billing_mode
                .or(sess_billing_mode.as_deref())
                .unwrap_or("")
                .to_string(),
        )
    };
    let ts = now();
    conn.execute(
        "INSERT INTO session_model_usage (
               session_id, model, billing_provider, billing_base_url, billing_mode,
               task, api_call_count, input_tokens, output_tokens,
               cache_read_tokens, cache_write_tokens, reasoning_tokens,
               estimated_cost_usd, actual_cost_usd, cost_status, cost_source,
               first_seen, last_seen
           ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(session_id, model, billing_provider, billing_base_url, billing_mode, task)
           DO UPDATE SET
               api_call_count = api_call_count + excluded.api_call_count,
               input_tokens = input_tokens + excluded.input_tokens,
               output_tokens = output_tokens + excluded.output_tokens,
               cache_read_tokens = cache_read_tokens + excluded.cache_read_tokens,
               cache_write_tokens = cache_write_tokens + excluded.cache_write_tokens,
               reasoning_tokens = reasoning_tokens + excluded.reasoning_tokens,
               estimated_cost_usd = estimated_cost_usd + excluded.estimated_cost_usd,
               actual_cost_usd = actual_cost_usd + excluded.actual_cost_usd,
               cost_status = COALESCE(excluded.cost_status, cost_status),
               cost_source = COALESCE(excluded.cost_source, cost_source),
               last_seen = excluded.last_seen",
        rusqlite::params![
            session_id,
            eff_model,
            eff_provider,
            eff_base_url,
            eff_billing_mode,
            task,
            api_call_count,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            reasoning_tokens,
            estimated_cost_usd.unwrap_or(0.0),
            actual_cost_usd.unwrap_or(0.0),
            cost_status,
            cost_source,
            ts,
            ts,
        ],
    )?;
    Ok(())
}

// ── coalescing ──────────────────────────────────────────────────────────────

#[derive(PartialEq, Eq, Debug, Clone)]
struct RouteKey {
    session_id: String,
    model: Option<String>,
    cost_status: Option<String>,
    cost_source: Option<String>,
    pricing_version: Option<String>,
    billing_provider: Option<String>,
    billing_base_url: Option<String>,
    billing_mode: Option<String>,
}

impl RouteKey {
    fn of(session_id: &str, delta: &TokenDelta) -> Self {
        RouteKey {
            session_id: session_id.to_string(),
            model: delta.model.clone(),
            cost_status: delta.cost_status.clone(),
            cost_source: delta.cost_source.clone(),
            pricing_version: delta.pricing_version.clone(),
            billing_provider: delta.billing_provider.clone(),
            billing_base_url: delta.billing_base_url.clone(),
            billing_mode: delta.billing_mode.clone(),
        }
    }
}

/// Merge consecutive incremental deltas with an identical route. Only
/// adjacent deltas merge; absolute (cumulative) deltas never merge.
///
/// PARITY: hermes_state.py _coalesce_token_deltas @ b9aa928 (5053–5082)
pub fn coalesce_token_deltas(batch: &[(String, TokenDelta)]) -> Vec<(String, TokenDelta)> {
    let mut groups: Vec<(Option<RouteKey>, String, TokenDelta)> = Vec::new();
    for (session_id, kwargs) in batch {
        let key = if !kwargs.absolute {
            Some(RouteKey::of(session_id, kwargs))
        } else {
            None
        };
        let mergeable = key.is_some()
            && groups
                .last()
                .map(|(last_key, _, _)| last_key.is_some() && last_key.as_ref() == key.as_ref())
                .unwrap_or(false);
        if mergeable {
            let merged = &mut groups.last_mut().unwrap().2;
            merged.input_tokens += kwargs.input_tokens;
            merged.output_tokens += kwargs.output_tokens;
            merged.cache_read_tokens += kwargs.cache_read_tokens;
            merged.cache_write_tokens += kwargs.cache_write_tokens;
            merged.reasoning_tokens += kwargs.reasoning_tokens;
            merged.api_call_count += kwargs.api_call_count;
            if let Some(v) = kwargs.estimated_cost_usd {
                merged.estimated_cost_usd = Some(merged.estimated_cost_usd.unwrap_or(0.0) + v);
            }
            if let Some(v) = kwargs.actual_cost_usd {
                merged.actual_cost_usd = Some(merged.actual_cost_usd.unwrap_or(0.0) + v);
            }
        } else {
            groups.push((key, session_id.clone(), kwargs.clone()));
        }
    }
    groups.into_iter().map(|(_, sid, kw)| (sid, kw)).collect()
}

// ── batch apply (never raises) ──────────────────────────────────────────────

/// Apply queued deltas in order on the given connection, coalescing where
/// safe. Never raises: per-delta failures are logged by the caller.
///
/// PARITY: hermes_state.py _apply_token_batch @ b9aa928 (5029–5052)
fn apply_token_batch_on(conn: &Connection, batch: &[(String, TokenDelta)]) {
    let coalesced = coalesce_token_deltas(batch);
    for (session_id, delta) in coalesced {
        let result = (|| -> Result<(), WriteError> {
            // Ensure the session row exists (separate write txn, like the
            // inline path) then apply the delta.
            let ensure = |c: &Connection| -> Result<(), WriteError> {
                insert_session_row_on(
                    c,
                    &session_id,
                    "unknown",
                    &NewSession {
                        model: delta.model.clone(),
                        ..Default::default()
                    },
                )
            };
            execute_write_on(conn, &ensure)?;
            let apply = |c: &Connection| -> Result<(), WriteError> {
                apply_token_update_on(c, &session_id, &delta)
            };
            execute_write_on(conn, &apply)
        })();
        if let Err(exc) = result {
            log_warn(&format!(
                "async token accounting: apply failed (session={}): {}",
                session_id, exc
            ));
        }
    }
}

// ── the writer thread ───────────────────────────────────────────────────────

fn token_writer_loop(db_path: PathBuf, shared: Arc<TokenWriterShared>, exit_tx: mpsc::Sender<()>) {
    let conn = match open_writer_conn(&db_path) {
        Ok(c) => c,
        Err(msg) => {
            log_warn(&format!(
                "async token accounting: writer connection failed: {}",
                msg
            ));
            shared.dead.store(true, AtomicOrdering::SeqCst);
            let _ = exit_tx.send(());
            return;
        }
    };
    loop {
        let batch = {
            let mut inner = lock_inner(&shared);
            while inner.queue.is_empty() && !inner.stop {
                inner = shared
                    .cond
                    .wait(inner)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            if inner.queue.is_empty() {
                // stop requested and fully drained
                break;
            }
            // busy is set BEFORE the queue is cleared: the lock-free fast
            // path in flush_token_counts() reads queue-then-busy, so no
            // observer can see an empty queue with a popped batch unapplied.
            inner.busy = true;
            let batch: Vec<(String, TokenDelta)> = inner.queue.drain(..).collect();
            batch
        };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply_token_batch_on(&conn, &batch);
        }));
        {
            let mut inner = lock_inner(&shared);
            inner.busy = false;
            shared.cond.notify_all();
        }
    }
    shared.dead.store(true, AtomicOrdering::SeqCst);
    let _ = exit_tx.send(());
}

// ── SessionDB token surface ─────────────────────────────────────────────────

impl SessionDB {
    /// Enqueue a token/cost delta for the background writer (async path).
    /// After `close()` has stopped the writer, falls back to the
    /// synchronous path and may raise like `update_token_counts`.
    ///
    /// PARITY: SessionDB.queue_token_counts @ b9aa928 (4913–4956)
    pub fn queue_token_counts(
        &self,
        session_id: &str,
        delta: TokenDelta,
    ) -> Result<(), WriteError> {
        let shared = self.token_writer.clone();
        {
            let mut inner = lock_inner(&shared);
            let thread_alive = {
                let guard = lock_thread(self);
                match guard.as_ref() {
                    Some(h) => !h.is_finished(),
                    None => false,
                }
            };
            let writer_stopped = inner.stop && !thread_alive;
            if !writer_stopped {
                inner.queue.push_back((session_id.to_string(), delta));
                if !thread_alive {
                    self.spawn_token_writer();
                }
                shared.cond.notify_all();
                return Ok(());
            }
        }
        // Writer permanently stopped (close() ran; a stop-flagged but
        // still-live writer keeps accepting — its loop drains before
        // exiting). Apply inline so a closed-connection failure raises at
        // the call site, exactly like the old synchronous path.
        self.update_token_counts(session_id, &delta)
    }

    /// Block until every queued token delta has been applied. Returns True
    /// when the queue is fully drained, False on timeout. Never raises:
    /// apply failures are logged.
    ///
    /// PARITY: SessionDB.flush_token_counts @ b9aa928 (4957–4999)
    pub fn flush_token_counts(&self, timeout: f64) -> bool {
        let shared = self.token_writer.clone();
        // Fast path — nothing queued, nothing in flight.
        {
            let inner = lock_inner(&shared);
            if inner.queue.is_empty() && !inner.busy {
                return true;
            }
        }
        let deadline = Instant::now() + Duration::from_secs_f64(timeout);
        let batch: Option<Vec<(String, TokenDelta)>> = {
            let mut inner = lock_inner(&shared);
            loop {
                // Natural exit: queue empty and idle — a live writer is
                // authoritative (upstream `while queue or busy` condition).
                if inner.queue.is_empty() && !inner.busy {
                    break None;
                }
                let thread_alive = {
                    let guard = lock_thread(self);
                    match guard.as_ref() {
                        Some(h) => !h.is_finished(),
                        None => false,
                    }
                };
                if !thread_alive && !inner.busy {
                    // Dead writer (or never started): the caller takes the
                    // leftovers. busy is claimed while draining (same
                    // protocol as the writer) so a concurrent flush cannot
                    // report drained while this batch is still unapplied.
                    inner.busy = true;
                    break Some(inner.queue.drain(..).collect::<Vec<_>>());
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return false;
                }
                let (guard, _) = shared
                    .cond
                    .wait_timeout(inner, remaining)
                    .unwrap_or_else(PoisonError::into_inner);
                inner = guard;
            }
        };
        if let Some(batch) = batch {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.apply_token_batch_sync(&batch);
            }));
            let mut inner = lock_inner(&shared);
            inner.busy = false;
            shared.cond.notify_all();
        }
        true
    }

    /// Stop the writer thread and drain remaining deltas. Never raises.
    ///
    /// PARITY: SessionDB._stop_token_writer @ b9aa928 (5083–5136)
    pub fn stop_token_writer(&self, join_timeout: f64) {
        let shared = self.token_writer.clone();
        {
            let mut inner = lock_inner(&shared);
            inner.stop = true;
            shared.cond.notify_all();
        }
        let handle = {
            let mut guard = lock_thread(self);
            guard.take()
        };
        if let Some(handle) = handle {
            if !handle.is_finished() {
                let exited = {
                    let rx_opt = self
                        .token_writer_exit
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .take();
                    match rx_opt {
                        Some(rx) => rx
                            .recv_timeout(Duration::from_secs_f64(join_timeout))
                            .is_ok(),
                        None => true,
                    }
                };
                if !exited {
                    log_warn(&format!(
                        "async token accounting: writer did not stop within {:.0}s; \
                         queued delta(s) may not be persisted",
                        join_timeout
                    ));
                    return;
                }
            }
            let _ = handle.join();
        }
        self.drain_token_leftovers(join_timeout);
    }

    /// Apply leftovers synchronously on the SessionDB's own connection,
    /// waiting out any concurrent caller-drain first (same busy protocol).
    ///
    /// PARITY: SessionDB._stop_token_writer leftover half @ b9aa928 (5100–5136)
    fn drain_token_leftovers(&self, join_timeout: f64) {
        let shared = self.token_writer.clone();
        let deadline = Instant::now() + Duration::from_secs_f64(join_timeout);
        let batch: Option<Vec<(String, TokenDelta)>> = {
            let mut inner = lock_inner(&shared);
            loop {
                if !inner.busy {
                    break;
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    log_warn(&format!(
                        "async token accounting: concurrent drain did not finish \
                         within {:.0}s; queued delta(s) not persisted",
                        join_timeout
                    ));
                    return;
                }
                let (guard, _) = shared
                    .cond
                    .wait_timeout(inner, remaining)
                    .unwrap_or_else(PoisonError::into_inner);
                inner = guard;
            }
            if inner.queue.is_empty() {
                None
            } else {
                // busy is claimed BEFORE the queue is cleared — same ordering
                // as the writer loop and the flush caller-drain.
                inner.busy = true;
                Some(inner.queue.drain(..).collect::<Vec<_>>())
            }
        };
        if let Some(batch) = batch {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.apply_token_batch_sync(&batch);
            }));
            let mut inner = lock_inner(&shared);
            inner.busy = false;
            shared.cond.notify_all();
        }
    }

    /// Apply a batch on the SessionDB's own connection (never raises).
    ///
    /// PARITY: SessionDB._apply_token_batch @ b9aa928 (5029–5052)
    fn apply_token_batch_sync(&self, batch: &[(String, TokenDelta)]) {
        let coalesced = coalesce_token_deltas(batch);
        for (session_id, delta) in coalesced {
            if let Err(exc) = self.update_token_counts(&session_id, &delta) {
                log_warn(&format!(
                    "async token accounting: apply failed (session={}): {}",
                    session_id, exc
                ));
            }
        }
    }

    /// Spawn (or respawn) the writer thread. Caller holds the queue lock.
    fn spawn_token_writer(&self) {
        let shared = self.token_writer.clone();
        let db_path = self.db_path.clone();
        let mut guard = lock_thread(self);
        // Recheck under the thread lock: another caller may have spawned.
        if let Some(h) = guard.as_ref() {
            if !h.is_finished() {
                return;
            }
        }
        let (exit_tx, exit_rx) = mpsc::channel();
        *self
            .token_writer_exit
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(exit_rx);
        shared.dead.store(false, AtomicOrdering::SeqCst);
        *guard = Some(
            std::thread::Builder::new()
                .name("session-db-token-writer".to_string())
                .spawn(move || token_writer_loop(db_path, shared, exit_tx))
                .expect("spawn token writer thread"),
        );
    }

    /// Synchronous token/cost accounting (the inline path).
    ///
    /// PARITY: SessionDB.update_token_counts @ b9aa928 (5143–5331)
    pub fn update_token_counts(
        &self,
        session_id: &str,
        delta: &TokenDelta,
    ) -> Result<(), WriteError> {
        // Ensure the session row exists so the UPDATE doesn't silently
        // affect 0 rows (INSERT OR IGNORE is cheap and idempotent).
        let ensure = |conn: &Connection| -> Result<(), WriteError> {
            insert_session_row_on(
                conn,
                session_id,
                "unknown",
                &NewSession {
                    model: delta.model.clone(),
                    ..Default::default()
                },
            )
        };
        self.execute_write(&ensure, None)?;
        let apply = |conn: &Connection| -> Result<(), WriteError> {
            apply_token_update_on(conn, session_id, delta)
        };
        self.execute_write(&apply, None)
    }

    /// Ensure a session row exists (INSERT OR IGNORE). Returns session_id.
    ///
    /// PARITY: SessionDB.ensure_session @ b9aa928 (5425–5444)
    pub fn ensure_session(
        &self,
        session_id: &str,
        source: &str,
        model: Option<String>,
    ) -> Result<String, WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            insert_session_row_on(
                conn,
                session_id,
                source,
                &NewSession {
                    model: model.clone(),
                    ..Default::default()
                },
            )
        };
        self.execute_write(&f, None)?;
        Ok(session_id.to_string())
    }

    /// Record an auxiliary LLM call's usage against session_id in
    /// `session_model_usage` under a task dimension, WITHOUT touching the
    /// sessions summary row. Best-effort by contract but returns errors for
    /// hard DB failures (unlike the writer path).
    ///
    /// PARITY: SessionDB.record_auxiliary_usage @ b9aa928 (5445–5483)
    #[allow(clippy::too_many_arguments)]
    pub fn record_auxiliary_usage(
        &self,
        session_id: &str,
        task: &str,
        model: Option<&str>,
        billing_provider: Option<&str>,
        billing_base_url: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
        reasoning_tokens: i64,
        estimated_cost_usd: Option<f64>,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() || task.is_empty() {
            return Ok(());
        }
        // FK guard: ensure the session row exists (same INSERT OR IGNORE
        // the update path uses — the initial create_session() can fail
        // under concurrent SQLite locking).
        let ensure = |conn: &Connection| -> Result<(), WriteError> {
            insert_session_row_on(conn, session_id, "unknown", &NewSession::default())
        };
        self.execute_write(&ensure, None)?;
        let apply = |conn: &Connection| -> Result<(), WriteError> {
            insert_model_usage_on(
                conn,
                session_id,
                model,
                billing_provider,
                billing_base_url,
                None, // aux rows never carry a billing_mode
                task,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
                reasoning_tokens,
                estimated_cost_usd,
                None, // aux rows never carry actual cost
                None, // cost_status
                None, // cost_source
                1,    // api_call_count hard-coded like upstream
            )
        };
        self.execute_write(&apply, None)
    }
}

// ── raw-connection surfaces (test oracle / internal reuse) ──────────────────

/// Apply a token batch on an arbitrary connection (used by parity tests and
/// the writer thread via `apply_token_batch_on`). Never raises.
#[doc(hidden)]
pub fn apply_token_batch_on_conn(conn: &Connection, batch: &[(String, TokenDelta)]) {
    apply_token_batch_on(conn, batch);
}

/// Connection-level `update_token_counts` (parity oracle: coalesced applies
/// must equal sequential applies on a fresh connection).
#[doc(hidden)]
pub fn update_token_counts_on_conn(
    conn: &Connection,
    session_id: &str,
    delta: &TokenDelta,
) -> Result<(), WriteError> {
    let ensure = |c: &Connection| -> Result<(), WriteError> {
        insert_session_row_on(
            c,
            session_id,
            "unknown",
            &NewSession {
                model: delta.model.clone(),
                ..Default::default()
            },
        )
    };
    execute_write_on(conn, &ensure)?;
    let apply =
        |c: &Connection| -> Result<(), WriteError> { apply_token_update_on(c, session_id, delta) };
    execute_write_on(conn, &apply)
}
