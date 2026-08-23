//! Parity oracles for the per-thread interrupt module, mirroring upstream
//! tests/tools/test_interrupt.py (TestInterruptModule cases) @ b9aa928.
//!
//! Excluded upstream cases (deferred subsystems — noted, not skipped):
//!  * TestPreToolCheck / TestRunToolCleanupOnBaseException — need
//!    run_agent.AIAgent._execute_tool_calls_* (agent crate not ported).
//!  * TestSIGKILLEscalation — needs tools/environments/local.py LocalEnvironment
//!    (sends real signals; environment crate not ported).
//!  * TestMessageCombining — pure queue-string joining, no module surface.
//!
//! Evidence tier: unit. Command: cargo test -p hermes-tools --test parity_interrupt

use std::sync::Mutex;
use std::thread;

use hermes_tools::interrupt::{
    clear_current_thread_interrupt, is_interrupted, set_interrupt, INTERRUPTED_THREADS,
    INTERRUPT_EVENT,
};

// The module's interrupted-thread set is process-global; serialize the tests
// that touch it so threads racing the shared set cannot see each other's
// state (cargo runs tests in the same binary concurrently).
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn set_and_check() {
    let _guard = TEST_LOCK.lock().unwrap();
    set_interrupt(false, None);
    assert!(!is_interrupted());

    set_interrupt(true, None);
    assert!(is_interrupted());

    set_interrupt(false, None);
    assert!(!is_interrupted());
}

#[test]
fn clear_current_thread_interrupt_leaves_other_threads() {
    // clear_current_thread_interrupt only touches the calling thread.
    let _guard = TEST_LOCK.lock().unwrap();
    INTERRUPTED_THREADS.lock().unwrap().clear();

    // An ident that isn't us — a real spawned thread's id (Python's test
    // fabricates `get_ident() + 1`; a live thread id is the Rust equivalent
    // and exercises the same "untouched elsewhere" contract).
    let other_tid = thread::spawn(|| thread::current().id()).join().unwrap();
    assert_ne!(other_tid, thread::current().id());

    set_interrupt(true, Some(other_tid));
    set_interrupt(true, None); // current thread
    assert!(is_interrupted());

    clear_current_thread_interrupt();

    assert!(!is_interrupted()); // ours cleared
    assert!(INTERRUPTED_THREADS.lock().unwrap().contains(&other_tid)); // other untouched
    INTERRUPTED_THREADS.lock().unwrap().remove(&other_tid);
}

#[test]
fn event_proxy_maps_to_per_thread_state() {
    // _interrupt_event shim: set/is_set/clear/wait round trip.
    let _guard = TEST_LOCK.lock().unwrap();
    set_interrupt(false, None);
    assert!(!INTERRUPT_EVENT.is_set());
    assert!(!INTERRUPT_EVENT.wait(None));

    INTERRUPT_EVENT.set();
    assert!(INTERRUPT_EVENT.is_set());
    assert!(INTERRUPT_EVENT.wait(Some(std::time::Duration::from_secs(1))));

    INTERRUPT_EVENT.clear();
    assert!(!INTERRUPT_EVENT.is_set());
}

#[test]
fn set_interrupt_targets_other_thread_without_touching_current() {
    // set_interrupt(active, Some(other)) must not disturb the current thread.
    let _guard = TEST_LOCK.lock().unwrap();
    INTERRUPTED_THREADS.lock().unwrap().clear();
    set_interrupt(false, None); // clean slate for this thread

    let other_tid = thread::spawn(|| thread::current().id()).join().unwrap();
    set_interrupt(true, Some(other_tid));
    assert!(!is_interrupted());
    assert!(INTERRUPTED_THREADS.lock().unwrap().contains(&other_tid));

    set_interrupt(false, Some(other_tid));
    assert!(!INTERRUPTED_THREADS.lock().unwrap().contains(&other_tid));
}
