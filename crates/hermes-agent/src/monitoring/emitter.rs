//! Monitoring emitter: fire-and-forget queue + background dispatcher.
//!
//! PARITY: `agent/monitoring/emitter.py` @ b9aa928 (whole module).
//!
//! The emitter is the single seam between producers (gateway status hooks,
//! the diagnostic log handler) and consumers (the OTLP streamers). Its
//! contract is the hot-path invariant:
//!
//! > `emit()` MUST return in O(microseconds), MUST NOT block on
//! > disk/network, and MUST NEVER raise into the caller. A monitoring
//! > failure is logged locally and dropped — it can never affect the
//! > gateway or a session.
//!
//! Mechanism: `emit` does a non-blocking bounded push; on a full queue it
//! drops the *oldest* event and counts the drop (bounded memory,
//! newest-wins). A daemon thread drains the queue and fans each batch out
//! to subscribers, fully fail-isolated. Nothing is persisted here — if no
//! subscriber is attached, events simply age out of the ring buffer.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Ring-buffer depth; oldest dropped when full.
/// PARITY: `_MAX_QUEUE` (upstream line 20).
pub const MAX_QUEUE: usize = 10_000;

/// PARITY: `_DRAIN_BATCH` (upstream line 21).
const DRAIN_BATCH: usize = 256;

/// A batch subscriber (the OTLP streamers). Called from the dispatcher
/// thread, fully fail-isolated.
pub type Subscriber = Arc<dyn Fn(&[Value]) + Send + Sync>;

struct Inner {
    queue: Mutex<InnerQueue>,
    queue_changed: Condvar,
    /// Signalled whenever the queue is empty and no batch is in flight.
    idle: Condvar,
    state: Mutex<InnerState>,
    stop: AtomicBool,
    enabled: AtomicBool,
    started: AtomicBool,
    subscribers: Mutex<Vec<Subscriber>>,
}

#[derive(Default)]
struct InnerQueue {
    items: VecDeque<Value>,
    in_flight: i64,
}

#[derive(Default)]
struct InnerState {
    dropped: i64,
    dispatched: i64,
}

/// Owns the queue, the dispatcher thread, and the subscriber list.
///
/// PARITY: `MonitoringEmitter` (upstream lines 24-153).
pub struct MonitoringEmitter {
    inner: Arc<Inner>,
    dispatcher: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Default for MonitoringEmitter {
    fn default() -> Self {
        Self::new(true)
    }
}

impl MonitoringEmitter {
    /// PARITY: `__init__(*, enabled: bool = True)` (upstream lines 25-41).
    pub fn new(enabled: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                queue: Mutex::new(InnerQueue::default()),
                queue_changed: Condvar::new(),
                idle: Condvar::new(),
                state: Mutex::new(InnerState::default()),
                stop: AtomicBool::new(false),
                enabled: AtomicBool::new(enabled),
                started: AtomicBool::new(false),
                subscribers: Mutex::new(Vec::new()),
            }),
            dispatcher: Mutex::new(None),
        }
    }

    /// Enqueue an event. Never blocks, never raises.
    ///
    /// `event` may be anything with `to_dict()` ([`ToMonitoringDict`]) or a
    /// plain `serde_json::Value` object (upstream: a dataclass with
    /// `to_dict()` or a plain dict). On a full queue the oldest event is
    /// dropped to make room.
    ///
    /// PARITY: `emit` (upstream lines 45-70) — the hot-path invariant.
    pub fn emit(&self, event: &dyn ToMonitoringDict) {
        if !self.inner.enabled.load(Ordering::SeqCst) {
            return;
        }
        let mut payload = event.to_dict();
        if let Some(map) = payload.as_object_mut() {
            // `payload.setdefault("ts_ns", time.time_ns())`.
            map.entry("ts_ns".to_string())
                .or_insert_with(|| json_now_ns());
        }
        self.enqueue_payload(payload);
    }

    /// Plain-dict overload of [`MonitoringEmitter::emit`].
    pub fn emit_value(&self, event: Value) {
        if !self.inner.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.enqueue_payload(event);
    }

    fn enqueue_payload(&self, mut payload: Value) {
        let Some(map) = payload.as_object_mut() else {
            return;
        };
        map.entry("ts_ns".to_string()).or_insert_with(json_now_ns);
        self.ensure_started();
        let mut queue = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        if queue.items.len() >= MAX_QUEUE {
            // Drop oldest to make room — bounded memory, newest-wins.
            queue.items.pop_front();
            self.inner
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .dropped += 1;
        }
        queue.items.push_back(payload);
        self.inner.queue_changed.notify_one();
    }

    fn ensure_started(&self) {
        if self.inner.started.load(Ordering::SeqCst) {
            return;
        }
        if self.inner.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let inner = Arc::clone(&self.inner);
        let handle = std::thread::Builder::new()
            .name("hermes-monitoring-dispatch".to_string())
            .spawn(move || dispatcher_loop(inner))
            .ok();
        *self.dispatcher.lock().unwrap_or_else(|e| e.into_inner()) = handle;
    }

    /// Register a live batch subscriber.
    ///
    /// PARITY: `subscribe` (upstream lines 118-122) — attaching the first
    /// subscriber also enables the emitter (collection is opt-in).
    pub fn subscribe(&self, callback: Subscriber) {
        let mut subs = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // `if callback not in self._subscribers` — identity dedup.
        if !subs.iter().any(|s| Arc::ptr_eq(s, &callback)) {
            subs.push(callback);
        }
        self.inner.enabled.store(true, Ordering::SeqCst);
    }

    /// PARITY: `unsubscribe` (upstream lines 124-129) — with no subscribers
    /// left, the emitter disables itself.
    pub fn unsubscribe(&self, callback: &Subscriber) {
        let mut subs = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = subs.iter().position(|s| Arc::ptr_eq(s, callback)) {
            subs.remove(pos);
        }
        if subs.is_empty() {
            self.inner.enabled.store(false, Ordering::SeqCst);
        }
    }

    /// Wait boundedly for queued and in-flight batches to finish dispatch.
    ///
    /// PARITY: `flush` (upstream lines 132-144). `timeout <= 0` returns
    /// immediately.
    pub fn flush(&self, timeout: f64) {
        if timeout <= 0.0 {
            return;
        }
        let deadline = Instant::now() + Duration::from_secs_f64(timeout);
        let mut queue = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        while !queue.items.is_empty() || queue.in_flight > 0 {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            let (guard, _timeout) = self
                .inner
                .idle
                .wait_timeout(queue, deadline - now)
                .unwrap_or_else(|e| e.into_inner());
            queue = guard;
        }
    }

    /// PARITY: `stats` (upstream lines 146-152).
    pub fn stats(&self) -> std::collections::HashMap<&'static str, i64> {
        let queue = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        let state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        let subs = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        [
            ("queued", queue.items.len() as i64),
            ("dispatched", state.dispatched),
            ("dropped", state.dropped),
            ("subscribers", subs.len() as i64),
        ]
        .into_iter()
        .collect()
    }

    /// PARITY: `close` (upstream lines 154-159).
    pub fn close(&self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        self.inner.queue_changed.notify_all();
        if let Some(handle) = self
            .dispatcher
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for MonitoringEmitter {
    fn drop(&mut self) {
        self.close();
    }
}

/// Marker for payloads carrying the upstream `to_dict()` contract.
pub trait ToMonitoringDict {
    fn to_dict(&self) -> Value;
}

impl ToMonitoringDict for super::events::GatewayHealthEvent {
    fn to_dict(&self) -> Value {
        super::events::GatewayHealthEvent::to_dict(self)
    }
}

impl ToMonitoringDict for super::events::GatewayDiagnosticEvent {
    fn to_dict(&self) -> Value {
        super::events::GatewayDiagnosticEvent::to_dict(self)
    }
}

impl ToMonitoringDict for super::events::CronExecutionEvent {
    fn to_dict(&self) -> Value {
        super::events::CronExecutionEvent::to_dict(self)
    }
}

impl<T: ?Sized + ToMonitoringDict> ToMonitoringDict for &T {
    fn to_dict(&self) -> Value {
        (**self).to_dict()
    }
}

fn json_now_ns() -> Value {
    Value::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0),
    )
}

/// PARITY: `_run` + `_dispatch` (upstream lines 84-116) — drain in batches
/// of [`DRAIN_BATCH`], fan out fully fail-isolated, then count dispatched.
fn dispatcher_loop(inner: Arc<Inner>) {
    loop {
        if inner.stop.load(Ordering::SeqCst) {
            return;
        }
        let batch = {
            let mut queue = inner.queue.lock().unwrap_or_else(|e| e.into_inner());
            // `self._q.get(timeout=0.5)` — wait up to 500ms for a first
            // item.
            let deadline = Instant::now() + Duration::from_millis(500);
            while queue.items.is_empty() {
                if inner.stop.load(Ordering::SeqCst) {
                    return;
                }
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let (guard, _) = inner
                    .queue_changed
                    .wait_timeout(queue, deadline - now)
                    .unwrap_or_else(|e| e.into_inner());
                queue = guard;
            }
            if queue.items.is_empty() {
                continue;
            }
            let take = queue.items.len().min(DRAIN_BATCH);
            let batch: Vec<Value> = queue.items.drain(..take).collect();
            queue.in_flight += batch.len() as i64;
            batch
        };
        // Fan-out to subscribers — fully fail-isolated.
        let subs = inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for sub in &subs {
            let sub = Arc::clone(sub);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sub(&batch)));
        }
        let mut state = inner.state.lock().unwrap_or_else(|e| e.into_inner());
        state.dispatched += batch.len() as i64;
        drop(state);
        let mut queue = inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.in_flight -= batch.len() as i64;
        if queue.items.is_empty() && queue.in_flight <= 0 {
            inner.idle.notify_all();
        }
    }
}

// ── process-wide singleton ──────────────────────────────────────────────────

static EMITTER: Mutex<Option<Arc<MonitoringEmitter>>> = Mutex::new(None);

/// Return the process-wide monitoring emitter.
///
/// Collection is opt-in: the singleton starts *disabled* and an exporter
/// enables it by attaching its first subscriber; until then producers are
/// no-ops.
///
/// PARITY: `get_emitter` (upstream lines 164-177).
pub fn get_emitter() -> Arc<MonitoringEmitter> {
    let mut guard = EMITTER.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(emitter) = guard.as_ref() {
        return Arc::clone(emitter);
    }
    let emitter = Arc::new(MonitoringEmitter::new(false));
    *guard = Some(Arc::clone(&emitter));
    emitter
}

/// Module-level convenience: emit via the singleton.
///
/// PARITY: `emit` (upstream lines 180-182).
pub fn emit(event: &dyn ToMonitoringDict) {
    get_emitter().emit(event);
}

/// Swap the singleton (tests only).
///
/// PARITY: `reset_emitter_for_tests` (upstream lines 185-193) — the
/// outgoing emitter is closed unless it is the one being installed.
pub fn reset_emitter_for_tests(emitter: Option<Arc<MonitoringEmitter>>) {
    let mut guard = EMITTER.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = guard.take() {
        let same = emitter
            .as_ref()
            .map(|new| Arc::ptr_eq(new, &existing))
            .unwrap_or(false);
        if !same {
            existing.close();
        }
    }
    *guard = emitter;
}

/// Back-compat alias for the salvaged class name used in emozilla's tests.
///
/// PARITY: `TelemetryEmitter = MonitoringEmitter` (upstream line 196).
pub type TelemetryEmitter = MonitoringEmitter;
