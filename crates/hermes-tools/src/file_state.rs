//! Cross-agent file state coordination.
//!
//! PARITY: `tools/file_state.py` @ 5d59366 (whole module, 254 lines).
//!
//! A process-wide singleton registry preventing mangled edits when
//! concurrent subagents touch the same file. Disabled by
//! `HERMES_DISABLE_FILE_STATE_GUARD=1` (re-read per call so tests can
//! toggle it).
//!
//! Locking: two levels like upstream — the meta table guards the
//! per-path lock entries, the state mutex guards reads + last-writer.
//! Per-path entries are refcounted and dropped once the last
//! holder/waiter exits, so the table cannot grow unbounded.
//! `parking_lot::lock_arc` gives an owned path guard with no
//! self-reference (the `Arc` clone inside keeps the mutex alive even
//! if the table entry is dropped while held).

use indexmap::IndexMap;
use parking_lot::{ArcMutexGuard, Mutex, RawMutex};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_PATHS_PER_AGENT: usize = 4096;
pub const MAX_GLOBAL_WRITERS: usize = 4096;

/// (mtime, read_ts, partial) — partial=True when read_file returned a
/// windowed view (offset > 1 or limit < total_lines): a later write
/// still warns so the model re-reads in full.
///
/// PARITY: `ReadStamp` (upstream lines 20-23).
pub type ReadStamp = (f64, f64, bool);

#[derive(Default)]
struct MetaTable {
    path_locks: HashMap<String, Arc<Mutex<()>>>,
    path_lock_users: HashMap<String, usize>,
}

pub struct FileStateRegistry {
    state: Mutex<State>,
    meta: Arc<Mutex<MetaTable>>,
}

#[derive(Default)]
struct State {
    reads: HashMap<String, IndexMap<String, ReadStamp>>,
    last_writer: IndexMap<String, (String, f64)>,
}

impl Default for FileStateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard for one held per-path lock. Dropping the last guard for a
/// path removes the lock-table entry (upstream's refcount arm: release
/// the path lock, then decrement under the meta lock).
pub struct PathGuard {
    _held: ArcMutexGuard<RawMutex, ()>,
    meta: Arc<Mutex<MetaTable>>,
    resolved: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        let mut meta = self.meta.lock();
        let users = meta
            .path_lock_users
            .get(&self.resolved)
            .copied()
            .unwrap_or(1);
        if users > 1 {
            meta.path_lock_users
                .insert(self.resolved.clone(), users - 1);
        } else {
            meta.path_lock_users.remove(&self.resolved);
            meta.path_locks.remove(&self.resolved);
        }
    }
}

impl FileStateRegistry {
    pub fn new() -> Self {
        FileStateRegistry {
            state: Mutex::new(State::default()),
            meta: Arc::new(Mutex::new(MetaTable::default())),
        }
    }

    /// Per-path lock: threads on the same path serialize, different
    /// paths proceed. The lock entry is dropped once the last
    /// holder/waiter exits.
    ///
    /// PARITY: `lock_path` (upstream lines 77-95).
    pub fn lock_path(&self, resolved: &str) -> PathGuard {
        let arc: Arc<Mutex<()>> = {
            let mut meta = self.meta.lock();
            let entry = meta
                .path_locks
                .entry(resolved.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone();
            *meta
                .path_lock_users
                .entry(resolved.to_string())
                .or_insert(0) += 1;
            entry
        };
        // Owned guard: keeps working even if the table entry is dropped
        // while held (the Arc clone pins the allocation).
        let held = parking_lot::Mutex::lock_arc(&arc);
        PathGuard {
            _held: held,
            meta: Arc::clone(&self.meta),
            resolved: resolved.to_string(),
        }
    }

    /// PARITY: `record_read` (upstream lines 103-111).
    pub fn record_read(&self, task_id: &str, resolved: &str, partial: bool, mtime: Option<f64>) {
        if disabled() {
            return;
        }
        let mtime = match mtime {
            Some(m) => m,
            None => match file_mtime(resolved) {
                Some(m) => m,
                None => return,
            },
        };
        let now = now_ts();
        let mut inner = self.state.lock();
        let agent_reads = inner.reads.entry(task_id.to_string()).or_default();
        agent_reads.insert(resolved.to_string(), (mtime, now, partial));
        evict_oldest(agent_reads, MAX_PATHS_PER_AGENT);
    }

    /// Record a successful write: global last-writer AND this agent's
    /// own read stamp (a write is an implicit read of the current
    /// content).
    ///
    /// PARITY: `note_write` (upstream lines 113-125).
    pub fn note_write(&self, task_id: &str, resolved: &str, mtime: Option<f64>) {
        if disabled() {
            return;
        }
        let mtime = match mtime {
            Some(m) => m,
            None => match file_mtime(resolved) {
                Some(m) => m,
                None => return,
            },
        };
        let now = now_ts();
        let mut inner = self.state.lock();
        inner
            .last_writer
            .insert(resolved.to_string(), (task_id.to_string(), now));
        evict_oldest(&mut inner.last_writer, MAX_GLOBAL_WRITERS);
        let agent_reads = inner.reads.entry(task_id.to_string()).or_default();
        agent_reads.insert(resolved.to_string(), (mtime, now, false));
        evict_oldest(agent_reads, MAX_PATHS_PER_AGENT);
    }

    /// Model-facing warning if this write would be stale, else `None`.
    /// Severity order: sibling wrote after our read > mtime drift /
    /// partial read > never read.
    ///
    /// PARITY: `check_stale` (upstream lines 127-175).
    pub fn check_stale(&self, task_id: &str, resolved: &str) -> Option<String> {
        if disabled() {
            return None;
        }
        let (stamp, last_writer) = {
            let inner = self.state.lock();
            (
                inner
                    .reads
                    .get(task_id)
                    .and_then(|r| r.get(resolved))
                    .copied(),
                inner.last_writer.get(resolved).cloned(),
            )
        };
        // Net-new file / first touch.
        if stamp.is_none() && last_writer.is_none() {
            return None;
        }
        // File doesn't exist — write creates it; not stale.
        let current_mtime = file_mtime(resolved)?;
        if let Some((writer_tid, writer_ts)) = &last_writer {
            if writer_tid != task_id {
                if let Some((_, read_ts, _)) = &stamp {
                    if *writer_ts > *read_ts {
                        return Some(format!(
                            "{} was modified by sibling subagent {writer_tid:?} at {} — after this agent's last read at {}. Re-read the file before writing.",
                            resolved, fmt_ts(*writer_ts), fmt_ts(*read_ts)
                        ));
                    }
                } else {
                    return Some(format!(
                        "{} was modified by sibling subagent {writer_tid:?} but this agent never read it. Read the file before writing to avoid overwriting the sibling's changes.",
                        resolved
                    ));
                }
            }
        }
        if let Some((read_mtime, _, partial)) = &stamp {
            if *read_mtime != current_mtime {
                return Some(format!(
                    "{resolved} was modified since you last read it on disk (external edit or unrecorded writer). Re-read the file before writing."
                ));
            }
            if *partial {
                return Some(format!(
                    "{resolved} was last read with offset/limit pagination (partial view). Read the remaining pages, or use patch, before overwriting it."
                ));
            }
            return None;
        }
        Some(format!(
            "{resolved} was not read by this agent. Read the file first so you can write an informed edit."
        ))
    }

    /// `{writer_task_id: [paths]}` for writes after `since_ts` by
    /// agents other than `exclude_task_id` (delegate_task's "subagent
    /// modified files you previously read" reminder).
    ///
    /// PARITY: `writes_since` (upstream lines 177-190).
    pub fn writes_since(
        &self,
        exclude_task_id: &str,
        since_ts: f64,
        paths: &[String],
    ) -> HashMap<String, Vec<String>> {
        if disabled() {
            return HashMap::new();
        }
        let paths_set: std::collections::HashSet<&String> = paths.iter().collect();
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        let inner = self.state.lock();
        for (p, (writer_tid, ts)) in &inner.last_writer {
            if writer_tid == exclude_task_id || *ts < since_ts || !paths_set.contains(p) {
                continue;
            }
            out.entry(writer_tid.clone()).or_default().push(p.clone());
        }
        out
    }

    /// Resolved paths this agent has read.
    ///
    /// PARITY: `known_reads` (upstream lines 192-197).
    pub fn known_reads(&self, task_id: &str) -> Vec<String> {
        if disabled() {
            return Vec::new();
        }
        let inner = self.state.lock();
        inner
            .reads
            .get(task_id)
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Release read stamps owned by a task after its lifecycle ends.
    ///
    /// PARITY: `forget_task` (upstream lines 199-202).
    pub fn forget_task(&self, task_id: &str) {
        let mut inner = self.state.lock();
        inner.reads.remove(task_id);
    }

    /// Reset all state. Intended for tests only.
    ///
    /// PARITY: `clear` (upstream lines 204-211).
    pub fn clear(&self) {
        self.state.lock().reads.clear();
        self.state.lock().last_writer.clear();
        self.meta.lock().path_locks.clear();
        self.meta.lock().path_lock_users.clear();
    }
}

static REGISTRY: OnceLock<FileStateRegistry> = OnceLock::new();

/// Process-wide singleton.
///
/// PARITY: `get_registry` (upstream lines 217-218).
pub fn get_registry() -> &'static FileStateRegistry {
    REGISTRY.get_or_init(FileStateRegistry::new)
}

/// True when the user switched the read-before-write guard off; the
/// file tools then warn instead of refusing stale/unread write_file
/// overwrites.
///
/// PARITY: `guard_disabled` (upstream lines 35-38).
pub fn guard_disabled() -> bool {
    disabled()
}

fn disabled() -> bool {
    // Re-read each call so tests can toggle via the environment.
    std::env::var("HERMES_DISABLE_FILE_STATE_GUARD")
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}

fn file_mtime(resolved: &str) -> Option<f64> {
    std::fs::metadata(resolved)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .ok()
}

fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn fmt_ts(ts: f64) -> String {
    // Short wall-clock for warnings; avoids datetime formatting on the
    // hot path (upstream `time.strftime("%H:%M:%S", localtime)`).
    let secs = ts.floor() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap())
        .with_timezone(&chrono::Local);
    dt.format("%H:%M:%S").to_string()
}

/// Pop entries until the map is within `cap`, oldest-inserted first.
/// `IndexMap` preserves insertion order, so `shift_remove_index(0)` is
/// the exact equivalent of upstream `_evict_oldest`'s dict arm
/// (upstream lines 53-63).
fn evict_oldest<K: Clone + Eq + std::hash::Hash, V>(map: &mut IndexMap<K, V>, cap: usize) {
    while map.len() > cap {
        map.shift_remove_index(0);
    }
}

// Re-export short names (call-site parity with the module-level wrappers).
pub fn record_read(task_id: &str, path: &str, partial: bool) {
    get_registry().record_read(task_id, path, partial, None);
}
pub fn note_write(task_id: &str, path: &str) {
    get_registry().note_write(task_id, path, None);
}
pub fn check_stale(task_id: &str, path: &str) -> Option<String> {
    get_registry().check_stale(task_id, path)
}
pub fn lock_path(path: &str) -> PathGuard {
    get_registry().lock_path(path)
}
pub fn writes_since(
    exclude_task_id: &str,
    since_ts: f64,
    paths: &[String],
) -> HashMap<String, Vec<String>> {
    get_registry().writes_since(exclude_task_id, since_ts, paths)
}
pub fn known_reads(task_id: &str) -> Vec<String> {
    get_registry().known_reads(task_id)
}
pub fn forget_task(task_id: &str) {
    get_registry().forget_task(task_id);
}
