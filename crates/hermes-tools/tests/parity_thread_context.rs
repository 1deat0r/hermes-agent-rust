//! Parity oracles for worker-thread context propagation, mirroring
//! tools/thread_context.py @ b9aa928 (upstream has NO dedicated test file —
//! the module code is the oracle; these tests mirror the documented
//! capture/install/clear lifecycle and the fail-closed comment).
//!
//! Evidence tier: unit. Command: cargo test -p hermes-tools --test parity_thread_context

use std::any::Any;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::thread;

use hermes_tools::thread_context::{
    get_approval_callback, get_sudo_password_callback, propagate_context_to_thread,
    set_approval_callback, set_context_snapshot_factory, set_sudo_password_callback,
};

type Callback = Arc<dyn Any + Send + Sync>;

fn marker() -> Callback {
    Arc::new(42_i32)
}

thread_local! {
    // Test-side marker the fake snapshot sets around `run` to prove the
    // wrapped target executes under the captured context.
    static IN_CTX: Cell<bool> = const { Cell::new(false) };
}

/// Fake context snapshot (mirrors what the approval/gateway contextvars layer
/// will install): marks the calling thread while `f` runs.
struct FakeSnapshot;
impl hermes_tools::thread_context::ContextSnapshot for FakeSnapshot {
    fn run(&self, f: Box<dyn FnOnce() + Send>) {
        IN_CTX.with(|cell| cell.set(true));
        f();
        IN_CTX.with(|cell| cell.set(false));
    }
}

#[test]
fn propagates_approval_callback_to_worker_and_clears() {
    set_approval_callback(Some(marker()));
    let wrapped = propagate_context_to_thread(|_: ()| get_approval_callback().is_some());

    let worker = thread::spawn(move || {
        let saw = wrapped(());
        // After the wrapped call returns, the worker's callback slot must be
        // cleared (upstream finally-clear; recycled threads never hold a
        // stale reference).
        let cleared = get_approval_callback().is_none();
        (saw, cleared)
    });
    let (saw, cleared) = worker.join().unwrap();
    assert!(saw, "worker should see the parent's approval callback");
    assert!(cleared, "worker's approval callback should be cleared on exit");

    // The parent thread's own slot is untouched.
    assert!(get_approval_callback().is_some());
    set_approval_callback(None);
}

#[test]
fn propagates_sudo_callback_roundtrip() {
    set_sudo_password_callback(Some(marker()));
    let wrapped = propagate_context_to_thread(|_: ()| {
        (get_sudo_password_callback().is_some(), get_approval_callback().is_none())
    });

    let worker = thread::spawn(move || {
        let (saw_sudo, no_approval) = wrapped(());
        let cleared = get_sudo_password_callback().is_none();
        (saw_sudo, no_approval, cleared)
    });
    let (saw_sudo, no_approval, cleared) = worker.join().unwrap();
    assert!(saw_sudo);
    assert!(no_approval, "unset parent callbacks are not installed");
    assert!(cleared);
    set_sudo_password_callback(None);
}

#[test]
fn no_callbacks_captured_still_runs_and_clears() {
    // Parent has no callbacks: wrapper must still invoke the target and
    // return its result.
    set_approval_callback(None);
    set_sudo_password_callback(None);
    let wrapped = propagate_context_to_thread(|x: i32| {
        let saw_any = get_approval_callback().is_none() && get_sudo_password_callback().is_none();
        (x * 2, saw_any)
    });

    let worker = thread::spawn(move || wrapped(21));
    let (doubled, saw_none) = worker.join().unwrap();
    assert_eq!(doubled, 42);
    assert!(saw_none);
}

#[test]
fn forwards_args_and_is_callable_multiple_times() {
    let wrapped = propagate_context_to_thread(|(a, b): (i32, i32)| a + b);
    assert_eq!(wrapped((1, 2)), 3);
    assert_eq!(wrapped((10, 20)), 30);
}

#[test]
fn runs_target_under_captured_context() {
    // Install the fake snapshot factory: the wrapped target must observe the
    // context entered during its run. Global seam — serialize.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    let _guard = TEST_LOCK.lock().unwrap();
    let factory: Arc<dyn Fn() -> Arc<dyn hermes_tools::thread_context::ContextSnapshot> + Send + Sync> =
        Arc::new(|| Arc::new(FakeSnapshot));
    set_context_snapshot_factory(Some(factory));

    let wrapped = propagate_context_to_thread(|_: ()| IN_CTX.with(|cell| cell.get()));
    let worker = thread::spawn(move || wrapped(()));
    let saw_context = worker.join().unwrap();
    assert!(saw_context, "target must run under the captured snapshot");

    // Outside the snapshot (after run returns) the marker is cleared.
    assert!(!IN_CTX.with(|cell| cell.get()));

    set_context_snapshot_factory(None);
    // Restore default: empty snapshot still runs targets.
    let wrapped = propagate_context_to_thread(|_: ()| IN_CTX.with(|cell| cell.get()));
    assert!(!wrapped(()));
}

#[test]
fn worker_thread_gets_its_own_empty_slots_by_default() {
    // A bare std::thread starts with no callback slots (the motivation for
    // propagate_context_to_thread in the first place).
    set_approval_callback(Some(marker()));
    let saw_main = get_approval_callback().is_some();
    let worker_sees = thread::spawn(|| get_approval_callback().is_some())
        .join()
        .unwrap();
    assert!(saw_main);
    assert!(!worker_sees, "bare workers start with empty callback slots");
    set_approval_callback(None);
}
