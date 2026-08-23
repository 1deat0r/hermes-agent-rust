//! Parity oracles for the daemon thread pool, mirroring upstream
//! tests/tools/test_daemon_pool.py @ b9aa928.
//!
//! Evidence tier: unit (in-process pool behavior; no live services).
//!
//! LANGUAGE-LEVEL NOTES (rule 4 gap report):
//! - `test_workers_are_daemon_threads` asserts `current_thread().daemon` and
//!   that the worker is absent from `concurrent.futures._threads_queues`.
//!   Rust threads have no daemon flag and no exit-hook registry; the daemon
//!   guarantee (workers never block process exit) holds by construction —
//!   `main` returning / `std::process::exit` tears down all threads without
//!   joining. The equivalent assertions here: the work runs on a non-main
//!   thread, and `shutdown(false)` never joins a wedged worker.
//! - `test_wedged_worker_does_not_block_interpreter_exit` runs a subprocess
//!   whose interpreter must exit while a worker sleeps 120s. The Rust
//!   equivalent holds by the same language guarantee; the mirrored test below
//!   verifies the pool's own teardown paths (`shutdown(false)`, `Drop`) do
//!   not join the wedged worker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hermes_tools::daemon_pool::DaemonThreadPoolExecutor;

const RESULT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
#[should_panic(expected = "max_workers must be greater than 0")]
fn zero_workers_matches_upstream_constructor_validation() {
    let _ = DaemonThreadPoolExecutor::new(0);
}

// Mirrors test_workers_are_daemon_threads: work runs on a pool worker thread
// (not the caller's thread), and the pool stays usable afterwards.
#[test]
fn work_runs_on_pool_worker_thread() {
    let pool = DaemonThreadPoolExecutor::new(2);
    let caller = thread::current().id();
    let worker = pool
        .submit(|| thread::current().id())
        .result(Some(RESULT_TIMEOUT))
        .expect("submit result");
    assert_ne!(worker, caller);
    // _threads_queues registration: no Rust equivalent (see file header).
    pool.shutdown(true);
}

// Mirrors test_idle_worker_reuse: sequential submits under light load are
// served by the same parked worker (no thread is spawned per task).
#[test]
fn idle_worker_reuse() {
    let pool = DaemonThreadPoolExecutor::new(4);
    let tid1 = pool
        .submit(|| thread::current().id())
        .result(Some(RESULT_TIMEOUT))
        .expect("first result");
    // Let the worker park on the idle wait, matching upstream's 0.05s sleep.
    thread::sleep(Duration::from_millis(50));
    let tid2 = pool
        .submit(|| thread::current().id())
        .result(Some(RESULT_TIMEOUT))
        .expect("second result");
    assert_eq!(tid1, tid2);
    pool.shutdown(true);
}

// Mirrors test_wedged_worker_does_not_block_interpreter_exit. In Rust the
// "interpreter exit" half is a language guarantee (process exit never joins
// threads); the pool-level contract being tested is that neither
// `shutdown(false)` nor `Drop` joins the wedged worker.
#[test]
fn wedged_worker_is_never_joined_by_teardown() {
    let pool = DaemonThreadPoolExecutor::new(1);
    pool.submit(|| thread::sleep(Duration::from_secs(120)));
    // Let the worker start the sleep before tearing down.
    thread::sleep(Duration::from_millis(300));

    let t0 = Instant::now();
    pool.shutdown(false);
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "shutdown(false) blocked on a wedged worker"
    );

    // Drop must behave like shutdown(wait=False) — no join, no block.
    let t1 = Instant::now();
    drop(pool);
    assert!(
        t1.elapsed() < Duration::from_secs(5),
        "Drop blocked on a wedged worker"
    );
}

// shutdown(wait=true) drains pending work before returning (the `with`-block
// / `shutdown(wait=True)` contract).
#[test]
fn shutdown_wait_runs_pending_work() {
    let pool = DaemonThreadPoolExecutor::new(2);
    let ran = Arc::new(AtomicBool::new(false));
    let ran2 = Arc::clone(&ran);
    let future = pool.submit(move || {
        ran2.store(true, Ordering::SeqCst);
        42
    });
    pool.shutdown(true);
    assert_eq!(future.result(None).expect("value"), 42);
    assert!(ran.load(Ordering::SeqCst));
}

// A second shutdown is a no-op (upstream `shutdown()` is idempotent).
#[test]
fn shutdown_is_idempotent() {
    let pool = DaemonThreadPoolExecutor::new(1);
    pool.shutdown(true);
    pool.shutdown(false);
    pool.shutdown(true);
}

// Scheduling after shutdown mirrors upstream RuntimeError -> panic.
#[test]
#[should_panic(expected = "cannot schedule new futures after shutdown")]
fn submit_after_shutdown_panics() {
    let pool = DaemonThreadPoolExecutor::new(1);
    pool.shutdown(false);
    let _ = pool.submit(|| 1);
}

// Supplementary (module-code oracle): the initializer runs once per worker.
#[test]
fn initializer_runs_in_worker() {
    let ran = Arc::new(AtomicBool::new(false));
    let ran2 = Arc::clone(&ran);
    let pool = DaemonThreadPoolExecutor::builder(1)
        .initializer(move || {
            ran2.store(true, Ordering::SeqCst);
        })
        .build();
    let value = pool.submit(|| 7).result(Some(RESULT_TIMEOUT)).expect("value");
    assert_eq!(value, 7);
    assert!(ran.load(Ordering::SeqCst), "initializer did not run");
    pool.shutdown(true);
}

// Supplementary: thread_name_prefix lands in the worker thread name.
#[test]
fn thread_name_prefix_is_used() {
    let pool = DaemonThreadPoolExecutor::builder(1)
        .thread_name_prefix("test-pool")
        .build();
    let name = pool
        .submit(|| thread::current().name().unwrap_or("").to_string())
        .result(Some(RESULT_TIMEOUT))
        .expect("value");
    assert!(
        name.starts_with("test-pool_"),
        "worker name was {name:?}"
    );
    pool.shutdown(true);
}

// Supplementary: max_workers caps concurrent workers (bounded fan-out).
#[test]
fn max_workers_is_respected() {
    let pool = DaemonThreadPoolExecutor::new(3);
    assert_eq!(pool.max_workers(), 3);
    let peak = Arc::new(Mutex::new(0usize));
    let running = Arc::new(Mutex::new(0usize));
    let mut futures = Vec::new();
    for _ in 0..6 {
        let peak2 = Arc::clone(&peak);
        let running2 = Arc::clone(&running);
        futures.push(pool.submit(move || {
            let mut r = running2.lock().unwrap();
            *r += 1;
            let mut p = peak2.lock().unwrap();
            if *r > *p {
                *p = *r;
            }
            drop(p);
            drop(r);
            thread::sleep(Duration::from_millis(30));
            *running2.lock().unwrap() -= 1;
        }));
    }
    for f in futures {
        f.result(Some(RESULT_TIMEOUT)).expect("drained");
    }
    assert_eq!(*peak.lock().unwrap(), 3, "workers exceeded max_workers");
    pool.shutdown(true);
}
