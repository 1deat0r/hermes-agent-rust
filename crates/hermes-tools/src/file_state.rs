//! Cross-agent file state coordination.
//!
//! PARITY: tools/file_state.py @ b9aa928 (332 LOC, ported 1:1).
//!
//! A process-wide singleton registry preventing mangled edits when
//! concurrent subagents touch the same file. Disabled by
//! `HERMES_DISABLE_FILE_STATE_GUARD=1`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_PATHS_PER_AGENT: usize = 4096;
pub const MAX_GLOBAL_WRITERS: usize = 4096;

/// (mtime, read_ts, partial) — partial means read_file returned a windowed
/// view, so writes after partial reads still warn.
pub type ReadStamp = (f64, f64, bool);

#[derive(Default)]
struct Inner {
    reads: HashMap<String, HashMap<String, ReadStamp>>,
    last_writer: HashMap<String, (String, f64)>,
}

pub struct FileStateRegistry {
    inner: Mutex<Inner>,
}

impl Default for FileStateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStateRegistry {
    pub fn new() -> Self {
        FileStateRegistry {
            inner: Mutex::new(Inner::default()),
        }
    }

    pub fn record_read(
        &self,
        task_id: &str,
        resolved: &str,
        partial: bool,
        mtime: Option<f64>,
    ) {
        if disabled() {
            return;
        }
        let mtime = match mtime {
            Some(m) => m,
            None => match std::fs::metadata(resolved).and_then(|m| m.modified()) {
                Ok(t) => t
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                Err(_) => return,
            },
        };
        let now = now_ts();
        let mut inner = self.inner.lock().expect("file state lock");
        let agent_reads = inner.reads.entry(task_id.to_string()).or_default();
        agent_reads.insert(resolved.to_string(), (mtime, now, partial));
        cap_dict(agent_reads, MAX_PATHS_PER_AGENT);
    }

    pub fn note_write(&self, task_id: &str, resolved: &str, mtime: Option<f64>) {
        if disabled() {
            return;
        }
        let mtime = match mtime {
            Some(m) => m,
            None => match std::fs::metadata(resolved).and_then(|m| m.modified()) {
                Ok(t) => t
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                Err(_) => return,
            },
        };
        let now = now_ts();
        let mut inner = self.inner.lock().expect("file state lock");
        inner.last_writer.insert(resolved.to_string(), (task_id.to_string(), now));
        cap_dict(&mut inner.last_writer, MAX_GLOBAL_WRITERS);
        let agent_reads = inner.reads.entry(task_id.to_string()).or_default();
        agent_reads.insert(resolved.to_string(), (mtime, now, false));
        cap_dict(agent_reads, MAX_PATHS_PER_AGENT);
    }

    pub fn check_stale(&self, task_id: &str, resolved: &str) -> Option<String> {
        if disabled() {
            return None;
        }
        let (stamp, last_writer) = {
            let inner = self.inner.lock().expect("file state lock");
            (
                inner.reads.get(task_id).and_then(|r| r.get(resolved)).copied(),
                inner.last_writer.get(resolved).cloned(),
            )
        };
        if stamp.is_none() && last_writer.is_none() {
            return None;
        }
        let current_mtime = match std::fs::metadata(resolved).and_then(|m| m.modified()) {
            Ok(t) => t
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            Err(_) => return None,
        };
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
                    "{resolved} was last read with offset/limit pagination (partial view). Re-read the whole file before overwriting it."
                ));
            }
        }
        if stamp.is_none() {
            return Some(format!(
                "{resolved} was not read by this agent. Read the file first so you can write an informed edit."
            ));
        }
        None
    }

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
        let inner = self.inner.lock().expect("file state lock");
        for (p, (writer_tid, ts)) in &inner.last_writer {
            if writer_tid == exclude_task_id || *ts < since_ts || !paths_set.contains(p) {
                continue;
            }
            out.entry(writer_tid.clone()).or_default().push(p.clone());
        }
        out
    }

    pub fn known_reads(&self, task_id: &str) -> Vec<String> {
        if disabled() {
            return Vec::new();
        }
        let inner = self.inner.lock().expect("file state lock");
        inner
            .reads
            .get(task_id)
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().expect("file state lock");
        inner.reads.clear();
        inner.last_writer.clear();
    }
}

static REGISTRY: OnceLock<FileStateRegistry> = OnceLock::new();

/// Process-wide singleton.
pub fn get_registry() -> &'static FileStateRegistry {
    REGISTRY.get_or_init(FileStateRegistry::new)
}

fn disabled() -> bool {
    std::env::var("HERMES_DISABLE_FILE_STATE_GUARD")
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}

fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn fmt_ts(ts: f64) -> String {
    let secs = ts.floor() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap())
        .with_timezone(&chrono::Local);
    dt.format("%H:%M:%S").to_string()
}

fn cap_dict<K: Clone + Eq + std::hash::Hash>(d: &mut HashMap<K, impl Sized>, limit: usize) {
    let over = d.len().saturating_sub(limit);
    if over == 0 {
        return;
    }
    // Drop insertion-order-oldest: HashMap has no insertion order, so a
    // FIFO approximation via key sampling keeps the cap bounded (documented
    // divergence from Python's ordered dict).
    let keys: Vec<K> = d.keys().cloned().collect();
    for k in keys.into_iter().take(over) {
        d.remove(&k);
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
pub fn writes_since(exclude_task_id: &str, since_ts: f64, paths: &[String]) -> HashMap<String, Vec<String>> {
    get_registry().writes_since(exclude_task_id, since_ts, paths)
}
pub fn known_reads(task_id: &str) -> Vec<String> {
    get_registry().known_reads(task_id)
}

// Keep the parity note: per-path locking (`lock_path`) is a caller-side
// critical-section concern in this port's single-process model; the registry
// lock covers the maps. Documented divergence until the executor layer
// provides task-level concurrency.
#[allow(dead_code)]
pub fn lock_path(_resolved: &str) {}
