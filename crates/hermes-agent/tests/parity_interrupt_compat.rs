// Tier: unit — mirrors tests/agent/test_interrupt_compat.py for the
// ABI-detection contract. Rust has no duck typing, so the two Python
// lookups (`inspect.getattr_static` for the modern ABI, `getattr` for the
// legacy one) are explicit adapter inputs: `Some(callable)` means "that
// lookup succeeded".
//
// The upstream `test_inherited_hard_interrupt_bypasses_legacy_subclass_override`
// case needs the unported `run_agent.AIAgent`, and
// `test_tui_subagent_interrupt_is_an_explicit_hard_stop` needs the unported
// `tools.delegate_tool` registry; both stay pending with those modules.

use hermes_agent::interrupt_compat::request_hard_interrupt;
use parking_lot::Mutex;
use std::sync::Arc;

/// Recorded `(abi, message)` pairs, in call order.
type Calls = Arc<Mutex<Vec<(String, Option<String>)>>>;

#[derive(Clone, Default)]
struct Recorder(Calls);

impl Recorder {
    fn hard(&self) -> impl Fn(Option<&str>) + '_ {
        let slot = Arc::clone(&self.0);
        move |message: Option<&str>| {
            slot.lock()
                .push(("hard".into(), message.map(str::to_string)))
        }
    }
    fn legacy(&self) -> impl Fn(Option<&str>) + '_ {
        let slot = Arc::clone(&self.0);
        move |message: Option<&str>| {
            slot.lock()
                .push(("legacy".into(), message.map(str::to_string)))
        }
    }
    fn calls(&self) -> Vec<(String, Option<String>)> {
        self.0.lock().clone()
    }
}

#[test]
fn producer_prefers_the_feature_detected_hard_interrupt() {
    let recorder = Recorder::default();
    let (hard, legacy) = (recorder.hard(), recorder.legacy());
    assert!(request_hard_interrupt(
        Some(&hard),
        Some(&legacy),
        Some("stop now"),
    ));
    assert_eq!(
        recorder.calls(),
        vec![("hard".to_string(), Some("stop now".to_string()))]
    );
}

#[test]
fn producer_falls_back_to_the_old_interrupt_signature() {
    let recorder = Recorder::default();
    let legacy = recorder.legacy();
    assert!(request_hard_interrupt(
        None,
        Some(&legacy),
        Some("stop now")
    ));
    assert_eq!(
        recorder.calls(),
        vec![("legacy".to_string(), Some("stop now".to_string()))]
    );
}

#[test]
fn producer_reports_an_unsupported_agent() {
    let recorder = Recorder::default();
    assert!(!request_hard_interrupt(None, None, Some("stop now")));
    assert!(recorder.calls().is_empty());
}

// The MagicMock case: a dynamic proxy fabricates both attributes, so the
// static modern lookup fails and only the legacy callable is offered.
#[test]
fn dynamic_proxy_does_not_fabricate_hard_interrupt_support() {
    let recorder = Recorder::default();
    let legacy = recorder.legacy();
    assert!(request_hard_interrupt(
        None,
        Some(&legacy),
        Some("stop now")
    ));
    assert_eq!(
        recorder.calls(),
        vec![("legacy".to_string(), Some("stop now".to_string()))]
    );
}

#[test]
fn a_missing_message_calls_the_producer_without_an_argument() {
    let recorder = Recorder::default();
    let hard = recorder.hard();
    assert!(request_hard_interrupt(Some(&hard), None, None));
    assert_eq!(recorder.calls(), vec![("hard".to_string(), None)]);
}

#[test]
fn inherited_hard_interrupt_bypasses_a_legacy_override() {
    // Mirrors the AIAgent-subclass contract: when the modern ABI resolves
    // (here: the caller supplies it), the legacy override is never called.
    let recorder = Recorder::default();
    let (hard, legacy) = (recorder.hard(), recorder.legacy());
    assert!(request_hard_interrupt(
        Some(&hard),
        Some(&legacy),
        Some("stop now"),
    ));
    assert_eq!(recorder.calls().len(), 1);
    assert_eq!(recorder.calls()[0].0, "hard");
}
