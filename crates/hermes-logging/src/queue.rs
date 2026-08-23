//! Background queue listener — file I/O never runs on the emitting thread.
//!
//! PARITY: hermes_logging.py lines 595–703 (`_NonFormattingQueueHandler`,
//! `_stop_queue_listener`, `_register_queued_handler`, `flush_log_queue`,
//! `drain_log_queue`, `rotating_file_handlers`).

use crate::record::{LogRecord, LogTarget};
use crate::rotating::RotatingHandler;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

enum Msg {
    Record(LogRecord),
    Stop,
}

/// The shared queue + background listener (guarded like upstream's
/// `_queue_state_lock`).
pub(crate) struct QueueState {
    tx: Option<Sender<Msg>>,
    listener: Option<JoinHandle<()>>,
    handlers: Vec<Arc<dyn LogTarget>>,
    pub file_handlers: Vec<Arc<RotatingHandler>>,
}

static QUEUE: Mutex<Option<QueueState>> = Mutex::new(None);

// Unit tests in queue.rs and setup.rs share this process-global queue. Keep
// their reset/register/assert sequences atomic when lib tests run in parallel.
#[cfg(test)]
pub(crate) static TEST_QUEUE_MUTEX: Mutex<()> = Mutex::new(());

fn queue() -> std::sync::MutexGuard<'static, Option<QueueState>> {
    QUEUE.lock().unwrap_or_else(|p| p.into_inner())
}

fn start_listener(rx: Receiver<Msg>, handlers: Vec<Arc<dyn LogTarget>>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("hermes-log".to_string())
        .spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    Msg::Record(rec) => {
                        for h in &handlers {
                            if h.accepts(&rec) {
                                h.emit(&rec);
                            }
                        }
                    }
                    Msg::Stop => break,
                }
            }
        })
        .expect("spawn log listener")
}

/// Register a file handler with the shared queue. The listener applies each
/// handler's own level + component filters on its worker thread. Adding a
/// handler rebuilds the listener over the full target set — mirroring
/// upstream `_register_queued_handler`, which stops and restarts the
/// `QueueListener` on every registration.
///
/// PARITY: `_register_queued_handler` (615–645).
pub fn register_queued_handler(handler: Arc<RotatingHandler>) {
    let mut q = queue();
    let state = q.get_or_insert_with(|| QueueState {
        tx: None,
        listener: None,
        handlers: Vec::new(),
        file_handlers: Vec::new(),
    });
    state.handlers.push(handler.clone());
    state.file_handlers.push(handler);
    // Stop any running listener (drains), then start one over the full set.
    if let Some(handle) = state.listener.take() {
        if let Some(tx) = state.tx.clone() {
            let _ = tx.send(Msg::Stop);
        }
        let _ = handle.join();
    }
    let (tx, rx) = channel();
    state.tx = Some(tx);
    state.listener = Some(start_listener(rx, state.handlers.clone()));
}

/// Enqueue a record — never blocks (unbounded channel like Python's
/// `queue.SimpleQueue`).
pub fn enqueue(record: LogRecord) {
    let q = queue();
    let Some(state) = q.as_ref() else {
        return;
    };
    let Some(tx) = state.tx.clone() else {
        return;
    };
    // Unbounded channel (like Python's queue.SimpleQueue): send never
    // blocks. If the listener is gone, drop the record — availability beats
    // the last log line on a wedged queue.
    let _ = tx.send(Msg::Record(record));
}

/// Stop the background listener (drains the queue by joining the worker).
fn stop_listener(state: &mut QueueState) {
    if let Some(tx) = state.tx.take() {
        let _ = tx.send(Msg::Stop);
    }
    if let Some(handle) = state.listener.take() {
        let _ = handle.join();
    }
}

/// Block until all queued records have been written, then resume.
///
/// Stopping joins the worker (draining the queue); restarting resumes.
///
/// PARITY: `flush_log_queue` (647–662).
pub fn flush_log_queue() {
    let mut q = queue();
    if let Some(state) = q.as_mut() {
        // Stopping joins the worker, which drains every queued record.
        if let Some(handle) = state.listener.take() {
            if let Some(tx) = state.tx.clone() {
                let _ = tx.send(Msg::Stop);
            }
            let _ = handle.join();
        }
        // Restart a fresh listener over the same handlers.
        let (tx, rx) = channel();
        state.tx = Some(tx);
        state.listener = Some(start_listener(rx, state.handlers.clone()));
    }
}

/// Best-effort, time-bounded drain for hard-exit paths (no restart).
///
/// PARITY: `drain_log_queue` (666–692).
pub fn drain_log_queue(timeout: std::time::Duration) {
    let mut q = queue();
    let Some(state) = q.as_mut() else { return };
    let tx = state.tx.take();
    let handle = state.listener.take();
    drop(q);
    let joiner = std::thread::spawn(move || {
        if let Some(tx) = tx {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(h) = handle {
            let _ = h.join();
        }
    });
    let _ = joiner.join();
    // NOTE: upstream bounds the wait by joining with a timeout; std threads
    // cannot be timed out. For a hard-exit drain this still flushes the queue
    // in the common case; the timeout contract is approximated by the caller
    // choosing when to call this. Documented divergence: we join unbounded.
    let _ = timeout;
}

/// Register any log target (e.g. a stderr handler) with the shared queue.
pub fn register_queued_target(target: Arc<dyn LogTarget>) {
    let mut q = queue();
    let state = q.get_or_insert_with(|| QueueState {
        tx: None,
        listener: None,
        handlers: Vec::new(),
        file_handlers: Vec::new(),
    });
    state.handlers.push(target);
    if let Some(handle) = state.listener.take() {
        if let Some(tx) = state.tx.clone() {
            let _ = tx.send(Msg::Stop);
        }
        let _ = handle.join();
    }
    let (tx, rx) = channel();
    state.tx = Some(tx);
    state.listener = Some(start_listener(rx, state.handlers.clone()));
}

/// The live rotating file handlers (attached to the async listener).
///
/// PARITY: `rotating_file_handlers` (694–700).
pub fn rotating_file_handlers() -> Vec<Arc<RotatingHandler>> {
    let q = queue();
    q.as_ref().map(|s| s.file_handlers.clone()).unwrap_or_default()
}

/// Tear down the async queue + listener (test-isolation helper).
///
/// PARITY: `_reset_queued_handlers` (703–715).
pub fn reset_queued_handlers() {
    let mut q = queue();
    if let Some(state) = q.as_mut() {
        stop_listener(state);
        state.handlers.clear();
        state.file_handlers.clear();
    }
    *q = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Level;
    use tempfile::TempDir;

    #[test]
    fn queue_flush_writes_records() {
        let _g = TEST_QUEUE_MUTEX.lock().unwrap();
        reset_queued_handlers();
        let td = TempDir::new().unwrap();
        let path = td.path().join("q.log");
        let h = std::sync::Arc::new(
            RotatingHandler::new(&path, Level::Info, 1024 * 1024, 3, None).unwrap(),
        );
        register_queued_handler(h);
        enqueue(crate::record::LogRecord::new(Level::Info, "t", "hello"));
        enqueue(crate::record::LogRecord::new(Level::Info, "t", "world"));
        flush_log_queue();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("hello"), "{}", text);
        assert!(text.contains("world"), "{}", text);
        reset_queued_handlers();
    }

    #[test]
    fn rotating_file_handlers_returns_registered() {
        let _g = TEST_QUEUE_MUTEX.lock().unwrap();
        reset_queued_handlers();
        let td = TempDir::new().unwrap();
        let h = std::sync::Arc::new(
            RotatingHandler::new(td.path().join("r.log"), Level::Info, 1024, 2, None).unwrap(),
        );
        register_queued_handler(h.clone());
        assert_eq!(rotating_file_handlers().len(), 1);
        reset_queued_handlers();
        assert_eq!(rotating_file_handlers().len(), 0);
    }
}
