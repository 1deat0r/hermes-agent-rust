//! Parity oracles for the slash-command confirmation primitive, mirroring
//! upstream tests/tools/test_slash_confirm.py @ b9aa928.
//! Evidence tier: unit (no external subsystems; the async handler seam is
//! exercised through boxed futures, see the module docs).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use hermes_tools::slash_confirm::{
    clear, clear_if_stale, get_pending, register, resolve, resolve_sync_compat,
    set_pending_created_at_for_test, SlashConfirmHandler, DEFAULT_TIMEOUT_SECONDS,
};

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

/// Wrap a plain sync callable as an async slash-confirm handler.
fn handler(
    f: impl Fn(String) -> Option<String> + Send + Sync + 'static,
) -> Arc<SlashConfirmHandler> {
    let f = Arc::new(f);
    Arc::new(move |choice: String| {
        let f = f.clone();
        Box::pin(async move { f(choice) })
    })
}

// Every test uses its own session_key: the state is process-global and the
// default Rust test harness runs tests in parallel.

#[test]
fn register_stores_entry() {
    let h = handler(|choice| Some(format!("got {choice}")));
    register("sess1", "cid1", "reload-mcp", h.clone());

    let pending = get_pending("sess1").expect("pending entry");
    assert_eq!(pending.confirm_id, "cid1");
    assert_eq!(pending.command, "reload-mcp");
    assert!(Arc::ptr_eq(&pending.handler, &h));
    assert!(pending.created_at > 0.0);
    clear("sess1");
}

#[test]
fn get_pending_returns_copy_not_reference() {
    let h = handler(|_| Some("x".to_string()));
    register("sess2", "cid1", "cmd", h);

    let mut p1 = get_pending("sess2").expect("pending entry");
    p1.command = "mutated".to_string();

    let p2 = get_pending("sess2").expect("pending entry");
    assert_eq!(p2.command, "cmd");
    clear("sess2");
}

#[test]
fn resolve_runs_handler_and_pops_entry() {
    let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let calls2 = calls.clone();
    let h = handler(move |choice| {
        calls2.lock().unwrap().push(choice.clone());
        Some(format!("resolved {choice}"))
    });
    register("sess3", "cid1", "reload-mcp", h);

    let result = resolve("sess3", "cid1", "once", DEFAULT_TIMEOUT_SECONDS);
    assert_eq!(result.as_deref(), Some("resolved once"));
    assert_eq!(*calls.lock().unwrap(), vec!["once".to_string()]);

    // Entry should be popped.
    assert!(get_pending("sess3").is_none());
}

#[test]
fn resolve_no_pending_returns_none() {
    assert!(resolve("nobody", "cid1", "once", DEFAULT_TIMEOUT_SECONDS).is_none());
}

#[test]
fn resolve_mismatched_confirm_id_returns_none() {
    let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let calls2 = calls.clone();
    let h = handler(move |choice| {
        calls2.lock().unwrap().push(choice);
        Some("ran".to_string())
    });
    register("sess4", "cid-new", "cmd", h);

    assert!(resolve("sess4", "cid-old", "once", DEFAULT_TIMEOUT_SECONDS).is_none());
    assert!(calls.lock().unwrap().is_empty());
    // A mismatched id does NOT pop the entry (superseded prompt check).
    assert!(get_pending("sess4").is_some());
    clear("sess4");
}

#[test]
fn resolve_superseded_confirm_returns_none() {
    let h1 = handler(|_| Some("one".to_string()));
    let h2 = handler(|_| Some("two".to_string()));
    register("sess5", "cid-1", "cmd", h1);
    // The user invoking a new confirmable command supersedes the stale one.
    register("sess5", "cid-2", "cmd", h2);

    assert!(resolve("sess5", "cid-1", "once", DEFAULT_TIMEOUT_SECONDS).is_none());
    let result = resolve("sess5", "cid-2", "once", DEFAULT_TIMEOUT_SECONDS);
    assert_eq!(result.as_deref(), Some("two"));
    assert!(get_pending("sess5").is_none());
}

#[test]
fn resolve_double_click_only_runs_handler_once() {
    let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let calls2 = calls.clone();
    let h = handler(move |choice| {
        calls2.lock().unwrap().push(choice);
        Some("ran".to_string())
    });
    register("sess6", "cid1", "cmd", h);

    // Simulate two near-simultaneous button clicks from two threads: the
    // entry is popped under the lock before the handler runs, so exactly
    // one resolve may run the handler.
    let (r1, r2) = std::thread::scope(|scope| {
        let a = scope.spawn(|| resolve("sess6", "cid1", "once", DEFAULT_TIMEOUT_SECONDS));
        let b = scope.spawn(|| resolve("sess6", "cid1", "once", DEFAULT_TIMEOUT_SECONDS));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(*calls.lock().unwrap(), vec!["once".to_string()]);
    let ran = (r1 == Some("ran".to_string())) as u8 + (r2 == Some("ran".to_string())) as u8;
    assert_eq!(ran, 1);
}

#[test]
fn resolve_stale_returns_none() {
    let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let calls2 = calls.clone();
    let h = handler(move |choice| {
        calls2.lock().unwrap().push(choice);
        Some("ran".to_string())
    });
    register("sess7", "cid1", "cmd", h);
    set_pending_created_at_for_test("sess7", now() - 10_000.0);

    assert!(resolve("sess7", "cid1", "once", 300.0).is_none());
    assert!(calls.lock().unwrap().is_empty());
    // Popped even though the resolve was dismissed as stale.
    assert!(get_pending("sess7").is_none());
}

#[test]
fn resolve_handler_none_result_returns_none() {
    let h = handler(|_| None);
    register("sess8", "cid1", "cmd", h);

    let result = resolve("sess8", "cid1", "cancel", DEFAULT_TIMEOUT_SECONDS);
    assert!(result.is_none());
}

#[test]
fn resolve_handler_panic_returns_error_string() {
    // Mirrors the upstream `except Exception` handler-failure path: the
    // result is a follow-up error string and the entry is already popped.
    // async blocks cannot declare a return type; the generic diversion
    // helper gives the block an `Option<String>` tail type so the panic
    // (upstream raises inside the awaited coroutine) still surfaces here.
    fn panic_never<T>(msg: &str) -> T {
        panic!("{msg}")
    }
    let h: Arc<SlashConfirmHandler> = Arc::new(
        |_choice: String| -> Pin<Box<dyn Future<Output = Option<String>> + Send>> {
            Box::pin(async move { panic_never::<Option<String>>("cancelled by user") })
        },
    );
    register("sess9", "cid1", "cmd", h);

    let result = resolve("sess9", "cid1", "once", DEFAULT_TIMEOUT_SECONDS);
    let result = result.expect("error string");
    assert!(
        result.contains("❌ Error handling confirmation: cancelled by user"),
        "{result}"
    );
    assert!(get_pending("sess9").is_none());
}

#[test]
fn clear_removes_entry() {
    let h = handler(|_| Some("x".to_string()));
    register("sess10", "cid1", "cmd", h);
    assert!(get_pending("sess10").is_some());

    clear("sess10");
    assert!(get_pending("sess10").is_none());
}

#[test]
fn clear_missing_is_noop() {
    clear("nobody"); // should not panic
}

#[test]
fn clear_if_stale_clears_stale_entry() {
    let h = handler(|_| Some("x".to_string()));
    register("sess11", "cid1", "cmd", h);
    set_pending_created_at_for_test("sess11", now() - 10_000.0);

    let cleared = clear_if_stale("sess11", 300.0);
    assert!(cleared);
    assert!(get_pending("sess11").is_none());
}

#[test]
fn clear_if_stale_keeps_fresh_entry() {
    let h = handler(|_| Some("x".to_string()));
    register("sess12", "cid1", "cmd", h);

    let cleared = clear_if_stale("sess12", 300.0);
    assert!(!cleared);
    assert!(get_pending("sess12").is_some());
    clear("sess12");
}

#[test]
fn clear_if_stale_returns_false_for_missing_entry() {
    assert!(!clear_if_stale("nobody", 300.0));
}

#[test]
fn resolve_sync_compat_delegates() {
    let h = handler(|choice| Some(format!("got {choice}")));
    register("sess13", "cid1", "cmd", h.clone());

    let result = resolve_sync_compat("sess13", "cid1", "always");
    assert_eq!(result.as_deref(), Some("got always"));
    assert!(get_pending("sess13").is_none());
}
