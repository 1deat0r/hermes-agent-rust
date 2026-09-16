//! Compatibility helper for explicit agent stop producers.
//!
//! PARITY: `agent/interrupt_compat.py` @ 5d59366 (whole module).
//!
//! New agents expose `hard_interrupt(message=None)`. Third-party agents and
//! old test doubles may only expose `interrupt(message=None)`; keep those
//! usable without sending the newer keyword they do not know. `tool_reason`
//! is a trusted, fixed category that may be exposed in model-visible tool
//! cancellation output, forwarded only when the callable explicitly supports
//! it. Returns `false` only when neither callable is available.
//!
//! Python resolves this by attribute lookup; Rust has no duck typing, so the
//! two lookups are explicit adapter arguments. The distinction the source
//! draws — `inspect.getattr_static` for the modern ABI versus plain `getattr`
//! for the legacy one — is the caller's job: pass `Some(..)` for
//! `hard_interrupt` only when the attribute exists *statically* on the
//! instance or its type, so a dynamic proxy (`MagicMock`, an RPC facade) that
//! fabricates any attribute still routes through the legacy arm.
//!
//! Likewise `_accepts_keyword` (an `inspect.signature` probe for the
//! `tool_reason` parameter or a `**kwargs` catch-all) has no Rust analog:
//! the caller states support with [`InterruptTarget::accepts_tool_reason`].

/// One resolved interrupt producer: `Fn(message, tool_reason)` matches
/// Python's `interrupt(message=None, *, tool_reason=None)` shape. The
/// `tool_reason` argument always arrives pre-gated — `None` unless the
/// caller supplied a reason *and* the target declares support — so producers
/// never need their own capability check.
pub type InterruptProducer<'a> = &'a dyn Fn(Option<&str>, Option<&str>);

/// A resolved interrupt target plus its declared `tool_reason` support.
pub struct InterruptTarget<'a> {
    /// The callable to invoke when this ABI wins.
    pub invoke: InterruptProducer<'a>,
    /// PARITY: `_accepts_keyword(callable, "tool_reason")` — true when the
    /// callable names `tool_reason` explicitly or accepts `**kwargs`.
    pub accepts_tool_reason: bool,
}

/// Request an explicit stop, preferring the modern hard-interrupt ABI.
///
/// PARITY: `request_hard_interrupt` (upstream lines 24-62).
pub fn request_hard_interrupt(
    hard_interrupt: Option<InterruptTarget<'_>>,
    interrupt: Option<InterruptTarget<'_>>,
    message: Option<&str>,
    tool_reason: Option<&str>,
) -> bool {
    let target = match (hard_interrupt, interrupt) {
        (Some(hard), _) => hard,
        (None, Some(legacy)) => legacy,
        (None, None) => return false,
    };
    // PARITY: `if tool_reason is not None and _accepts_keyword(...)`.
    let reason = match tool_reason {
        Some(reason) if target.accepts_tool_reason => Some(reason),
        _ => None,
    };
    (target.invoke)(message, reason);
    true
}
