// Tier: unit — mirrors tests/agent/test_interrupt_compat.py for the
// ABI-detection contract. Rust has no duck typing, so the two Python
// lookups (`inspect.getattr_static` for the modern ABI, `getattr` for the
// legacy one) are explicit adapter inputs: `Some(target)` means "that
// lookup succeeded". Likewise `_accepts_keyword` (an `inspect.signature`
// probe) is an explicit `accepts_tool_reason` flag on each target.
//
// The upstream `test_inherited_hard_interrupt_bypasses_legacy_subclass_override`
// case needs the unported `run_agent.AIAgent`, and
// `test_tui_subagent_interrupt_is_an_explicit_hard_stop` needs the unported
// `tools.delegate_tool` registry; both stay pending with those modules.

use hermes_agent::interrupt_compat::{request_hard_interrupt, InterruptTarget};
use parking_lot::Mutex;
use std::sync::Arc;

/// Recorded `(abi, message, tool_reason)` triples, in call order.
type Calls = Arc<Mutex<Vec<(String, Option<String>, Option<String>)>>>;

#[derive(Clone, Default)]
struct Recorder(Calls);

impl Recorder {
    fn hard(&self, accepts_tool_reason: bool) -> (impl Fn(Option<&str>, Option<&str>) + '_, bool) {
        let slot = Arc::clone(&self.0);
        let invoke = move |message: Option<&str>, tool_reason: Option<&str>| {
            slot.lock().push((
                "hard".into(),
                message.map(str::to_string),
                tool_reason.map(str::to_string),
            ))
        };
        (invoke, accepts_tool_reason)
    }
    fn legacy(
        &self,
        accepts_tool_reason: bool,
    ) -> (impl Fn(Option<&str>, Option<&str>) + '_, bool) {
        let slot = Arc::clone(&self.0);
        let invoke = move |message: Option<&str>, tool_reason: Option<&str>| {
            slot.lock().push((
                "legacy".into(),
                message.map(str::to_string),
                tool_reason.map(str::to_string),
            ))
        };
        (invoke, accepts_tool_reason)
    }
    fn calls(&self) -> Vec<(String, Option<String>, Option<String>)> {
        self.0.lock().clone()
    }
}

fn target<'a>(
    invoke: &'a dyn Fn(Option<&str>, Option<&str>),
    accepts: bool,
) -> InterruptTarget<'a> {
    InterruptTarget {
        invoke,
        accepts_tool_reason: accepts,
    }
}

#[test]
fn producer_prefers_the_feature_detected_hard_interrupt() {
    let recorder = Recorder::default();
    let (hard_invoke, _) = recorder.hard(false);
    let (legacy_invoke, _) = recorder.legacy(false);
    assert!(request_hard_interrupt(
        Some(target(&hard_invoke, false)),
        Some(target(&legacy_invoke, false)),
        Some("stop now"),
        None,
    ));
    assert_eq!(
        recorder.calls(),
        vec![("hard".to_string(), Some("stop now".to_string()), None)]
    );
}

#[test]
fn producer_falls_back_to_the_old_interrupt_signature() {
    let recorder = Recorder::default();
    let (legacy_invoke, _) = recorder.legacy(false);
    assert!(request_hard_interrupt(
        None,
        Some(target(&legacy_invoke, false)),
        Some("stop now"),
        None,
    ));
    assert_eq!(
        recorder.calls(),
        vec![("legacy".to_string(), Some("stop now".to_string()), None)]
    );
}

#[test]
fn producer_reports_an_unsupported_agent() {
    let recorder = Recorder::default();
    assert!(!request_hard_interrupt(None, None, Some("stop now"), None));
    assert!(recorder.calls().is_empty());
}

// The MagicMock case: a dynamic proxy fabricates both attributes, so the
// static modern lookup fails and only the legacy callable is offered.
#[test]
fn dynamic_proxy_does_not_fabricate_hard_interrupt_support() {
    let recorder = Recorder::default();
    let (legacy_invoke, _) = recorder.legacy(false);
    assert!(request_hard_interrupt(
        None,
        Some(target(&legacy_invoke, false)),
        Some("stop now"),
        None,
    ));
    assert_eq!(
        recorder.calls(),
        vec![("legacy".to_string(), Some("stop now".to_string()), None)]
    );
}

#[test]
fn a_missing_message_calls_the_producer_without_an_argument() {
    let recorder = Recorder::default();
    let (hard_invoke, _) = recorder.hard(false);
    assert!(request_hard_interrupt(
        Some(target(&hard_invoke, false)),
        None,
        None,
        None
    ));
    assert_eq!(recorder.calls(), vec![("hard".to_string(), None, None)]);
}

#[test]
fn inherited_hard_interrupt_bypasses_a_legacy_override() {
    // Mirrors the AIAgent-subclass contract: when the modern ABI resolves
    // (here: the caller supplies it), the legacy override is never called.
    let recorder = Recorder::default();
    let (hard_invoke, _) = recorder.hard(false);
    let (legacy_invoke, _) = recorder.legacy(false);
    assert!(request_hard_interrupt(
        Some(target(&hard_invoke, false)),
        Some(target(&legacy_invoke, false)),
        Some("stop now"),
        None,
    ));
    assert_eq!(recorder.calls().len(), 1);
    assert_eq!(recorder.calls()[0].0, "hard");
}

#[test]
fn safe_tool_reason_only_reaches_a_supporting_modern_agent() {
    // PARITY: `test_safe_tool_reason_only_reaches_supporting_modern_agent`
    // (`tests/agent/test_interrupt_compat.py` @ 5d59366). The modern agent
    // declares `tool_reason` in its signature; the legacy one does not, so
    // the trusted category is forwarded to the former and withheld from the
    // latter (upstream: `_accepts_keyword` gate).
    let modern = Recorder::default();
    let legacy = Recorder::default();
    let (modern_invoke, _) = modern.hard(true);
    let (legacy_invoke, _) = legacy.legacy(false);
    assert!(request_hard_interrupt(
        Some(target(&modern_invoke, true)),
        None,
        Some("private diagnostic"),
        Some("fixed category"),
    ));
    assert!(request_hard_interrupt(
        None,
        Some(target(&legacy_invoke, false)),
        Some("private diagnostic"),
        Some("fixed category"),
    ));
    assert_eq!(
        modern.calls(),
        vec![(
            "hard".to_string(),
            Some("private diagnostic".to_string()),
            Some("fixed category".to_string())
        )]
    );
    assert_eq!(
        legacy.calls(),
        vec![(
            "legacy".to_string(),
            Some("private diagnostic".to_string()),
            None
        )]
    );
}

#[test]
fn tool_reason_without_support_is_withheld_but_still_stops() {
    // Hardening: an unsupported target still performs the stop; only the
    // model-visible category is withheld.
    let recorder = Recorder::default();
    let (hard_invoke, _) = recorder.hard(false);
    assert!(request_hard_interrupt(
        Some(target(&hard_invoke, false)),
        None,
        Some("stop now"),
        Some("fixed category"),
    ));
    assert_eq!(
        recorder.calls(),
        vec![("hard".to_string(), Some("stop now".to_string()), None)]
    );
}
