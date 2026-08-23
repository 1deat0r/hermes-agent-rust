//! Shared daemon-thread thread pool.
//!
//! PARITY: tools/daemon_pool.py @ b9aa928 (64 LOC, ported 1:1).
//!
//! Upstream motivation: stdlib `ThreadPoolExecutor` workers are non-daemon
//! AND are registered in `concurrent.futures.thread._threads_queues`, whose
//! atexit hook (`_python_exit`) joins every worker unconditionally — even
//! after `shutdown(wait=False)`. A single wedged worker (tool blocked on
//! network I/O, hung provider daemon, stuck subagent) therefore blocks
//! interpreter exit forever. `DaemonThreadPoolExecutor` spawns daemon workers
//! and skips the `_threads_queues` registration.
//!
//! Rust threads have no daemon flag and are never joined at process exit:
//! when `main` returns (or `std::process::exit` is called), every thread is
//! torn down without a join. The daemon guarantee therefore holds by
//! construction in Rust; this type mirrors the remaining pool contract —
//! bounded worker count, idle-thread reuse, initializer/initargs, `submit`
//! futures, `shutdown(wait)` semantics, and a `Drop` path that never blocks
//! on a wedged worker (mirroring how upstream abandons pools deliberately).
//!
//! Use it for any pool whose work is best-effort or independently
//! interruptible and must never hold the process open: concurrent tool
//! execution, background memory sync, catalog fan-out, subagent timeout
//! wrappers. Do NOT use it for work that must complete before exit (durable
//! writes) — those belong on foreground threads with explicit bounded joins.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

type Job = Box<dyn FnOnce() + Send>;
type Initializer = Box<dyn Fn() + Send + Sync>;

struct Inner {
    /// Pending work, guarded so multiple workers can pop safely.
    queue: Mutex<VecDeque<Job>>,
    /// Wakes parked workers when work is enqueued or shutdown is requested.
    has_work: Condvar,
    /// Number of workers currently parked waiting for work. Mirrors CPython's
    /// `_idle_semaphore` for the spawn-decision in `adjust_thread_count`.
    idle_workers: Mutex<usize>,
    /// Live worker join handles (only consulted by `shutdown(wait=true)`).
    threads: Mutex<Vec<JoinHandle<()>>>,
    shutdown: AtomicBool,
    max_workers: usize,
    thread_name_prefix: String,
    initializer: Option<Initializer>,
}

/// ThreadPoolExecutor variant whose workers do not block process exit.
pub struct DaemonThreadPoolExecutor {
    inner: Arc<Inner>,
}

/// Handle to a submitted call, mirroring `concurrent.futures.Future`.
pub struct DaemonFuture<R> {
    rx: mpsc::Receiver<R>,
}

impl<R> DaemonFuture<R> {
    /// Block until the wrapped call completes and return its value.
    ///
    /// `timeout: None` waits forever (upstream default `timeout=None`); with
    /// a timeout, `Err(RecvTimeoutError::Timeout)` mirrors upstream
    /// `concurrent.futures.TimeoutError`. A job that panicked before sending
    /// its result surfaces as `Err(Disconnected)` — the Rust mirror of the
    /// upstream future raising the worker exception.
    pub fn result(self, timeout: Option<Duration>) -> Result<R, mpsc::RecvTimeoutError> {
        match timeout {
            Some(t) => self.rx.recv_timeout(t),
            None => self.rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        }
    }
}

impl DaemonThreadPoolExecutor {
    /// Create a pool with `max_workers` workers.
    pub fn new(max_workers: usize) -> Self {
        DaemonThreadPoolExecutor::builder(max_workers).build()
    }

    /// Configure a pool (thread name prefix / initializer) mirroring the
    /// upstream `thread_name_prefix=` / `initializer=` constructor kwargs.
    pub fn builder(max_workers: usize) -> DaemonPoolBuilder {
        DaemonPoolBuilder {
            max_workers,
            thread_name_prefix: String::new(),
            initializer: None,
        }
    }

    /// Schedule a call to run on the pool; returns a `DaemonFuture`.
    ///
    /// Callers capture arguments in the closure (the Rust equivalent of
    /// passing `*args` to `Future.submit`).
    pub fn submit<F, R>(&self, f: F) -> DaemonFuture<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // PARITY: upstream raises RuntimeError("cannot schedule new futures
        // after shutdown"); a panic mirrors the raise.
        if self.inner.shutdown.load(Ordering::SeqCst) {
            panic!("cannot schedule new futures after shutdown");
        }
        self.adjust_thread_count();
        let (tx, rx) = mpsc::channel::<R>();
        let job: Job = Box::new(move || {
            let _ = tx.send(f());
        });
        self.inner.queue.lock().unwrap().push_back(job);
        self.inner.has_work.notify_one();
        DaemonFuture { rx }
    }

    /// Shut the pool down. With `wait=true`, block until all pending calls
    /// have run and every worker has exited (upstream `shutdown(wait=True)` /
    /// the `with`-block exit). With `wait=false`, request shutdown without
    /// joining — workers still drain the pending queue, but a wedged worker
    /// is never joined and cannot hold the process open.
    pub fn shutdown(&self, wait: bool) {
        if !self.inner.shutdown.swap(true, Ordering::SeqCst) {
            // New shutdown: wake parked workers so they observe the flag.
            self.inner.has_work.notify_all();
        }
        if wait {
            let handles = std::mem::take(&mut *self.inner.threads.lock().unwrap());
            for handle in handles {
                // PARITY: `shutdown(wait=True)` joins workers even when one
                // is wedged — exactly like the upstream `with` block.
                let _ = handle.join();
            }
        }
    }

    /// Number of workers this pool may spawn (exposed for tests/introspection).
    pub fn max_workers(&self) -> usize {
        self.inner.max_workers
    }

    // PARITY: tools/daemon_pool.py `_adjust_thread_count` — mirrors CPython's
    // implementation (3.8–3.13) with daemon=True and no `_threads_queues`
    // registration. Spawn a new worker only when no idle worker is parked and
    // we are below `max_workers`.
    fn adjust_thread_count(&self) {
        // Equivalent of `_idle_semaphore.acquire(timeout=0)`: if an idle
        // worker is parked it will pick the job up, so do not spawn.
        {
            let idle = self.inner.idle_workers.lock().unwrap();
            if *idle > 0 {
                return;
            }
        }
        let mut threads = self.inner.threads.lock().unwrap();
        if threads.len() < self.inner.max_workers {
            let prefix = if self.inner.thread_name_prefix.is_empty() {
                "DaemonThreadPoolExecutor"
            } else {
                &self.inner.thread_name_prefix
            };
            let thread_name = format!("{prefix}_{}", threads.len());
            let inner = Arc::clone(&self.inner);
            let handle = std::thread::Builder::new()
                .name(thread_name)
                .spawn(move || worker_loop(inner))
                .expect("failed to spawn daemon pool worker");
            threads.push(handle);
        }
    }
}

/// Builder mirroring the upstream constructor kwargs.
pub struct DaemonPoolBuilder {
    max_workers: usize,
    thread_name_prefix: String,
    initializer: Option<Initializer>,
}

impl DaemonPoolBuilder {
    /// Set `thread_name_prefix` (aids debugging in stack dumps).
    pub fn thread_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.thread_name_prefix = prefix.into();
        self
    }

    /// Install a per-worker initializer (upstream `initializer=` kwarg; Rust
    /// closures capture `initargs`).
    pub fn initializer<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.initializer = Some(Box::new(f));
        self
    }

    pub fn build(self) -> DaemonThreadPoolExecutor {
        // PARITY: concurrent.futures.ThreadPoolExecutor rejects zero or
        // negative max_workers with ValueError. `usize` cannot represent a
        // negative value; panic is the Rust equivalent for zero.
        assert!(self.max_workers > 0, "max_workers must be greater than 0");
        DaemonThreadPoolExecutor {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::new()),
                has_work: Condvar::new(),
                idle_workers: Mutex::new(0),
                threads: Mutex::new(Vec::new()),
                shutdown: AtomicBool::new(false),
                max_workers: self.max_workers,
                thread_name_prefix: self.thread_name_prefix,
                initializer: self.initializer,
            }),
        }
    }
}

/// Dropping a pool never joins its workers (the daemon contract): the pool
/// requests shutdown without waiting, so a wedged worker can never block
/// process exit. Pending work still drains on the worker threads.
///
/// PARITY DIVERGENCE (documented): upstream `DaemonThreadPoolExecutor` gets
/// `ThreadPoolExecutor.__del__` which calls `shutdown(wait=True)` — but that
/// path is deliberately avoided in practice by leaking pools that outlive
/// their useful life. In Rust an explicit `Drop` must exist, and the daemon
/// spirit (never hold the process open) wins: drop behaves like
/// `shutdown(wait=False)`. Callers who need join semantics call
/// `shutdown(true)` explicitly (the `with`-block equivalent).
impl Drop for DaemonThreadPoolExecutor {
    fn drop(&mut self) {
        self.shutdown(false);
    }
}

fn worker_loop(inner: Arc<Inner>) {
    if let Some(initializer) = &inner.initializer {
        initializer();
    }
    loop {
        let job = {
            let mut queue = inner.queue.lock().unwrap();
            // Park: announce ourselves idle for `_adjust_thread_count`'s
            // spawn decision, matching CPython's `_idle_semaphore.release()`.
            *inner.idle_workers.lock().unwrap() += 1;
            loop {
                if inner.shutdown.load(Ordering::SeqCst) && queue.is_empty() {
                    *inner.idle_workers.lock().unwrap() -= 1;
                    return;
                }
                match queue.pop_front() {
                    Some(job) => {
                        // Claimed a job: not idle any more.
                        *inner.idle_workers.lock().unwrap() -= 1;
                        break Some(job);
                    }
                    None => {
                        // Wait for work (or shutdown). Mirrors CPython's
                        // `_work_queue.get(block=True)` + sentinel wake-up.
                        queue = inner.has_work.wait(queue).unwrap();
                    }
                }
            }
        };
        // Run the job with the queue lock released.
        if let Some(job) = job {
            job();
        }
    }
}
