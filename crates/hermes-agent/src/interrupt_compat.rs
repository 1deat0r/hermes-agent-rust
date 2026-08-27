//! Compatibility helper for explicit agent stop producers.
//!
//! PARITY: `agent/interrupt_compat.py` @ b9aa928 (whole module, lines 1-36).
//!
//! New agents expose `hard_interrupt(message=None)`. Third-party agents and
//! old test doubles may only expose `interrupt(message=None)`; keep those
//! usable without sending the newer keyword they do not know. Returns `false`
//! only when neither callable is available.
//!
//! Python resolves this by attribute lookup; Rust has no duck typing, so the
//! two lookups are explicit adapter arguments. The distinction the source
//! draws — `inspect.getattr_static` for the modern ABI versus plain `getattr`
//! for the legacy one — is the caller's job: pass `Some(..)` for
//! `hard_interrupt` only when the attribute exists *statically* on the
//! instance or its type, so a dynamic proxy (`MagicMock`, an RPC facade) that
//! fabricates any attribute still routes through the legacy arm.

/// One resolved interrupt producer: `Fn(Option<&str>)` matches Python's
/// `interrupt(message=None)` / `hard_interrupt(message=None)` signatures.
pub type InterruptProducer<'a> = &'a dyn Fn(Option<&str>);

/// Request an explicit stop, preferring the modern hard-interrupt ABI.
///
/// PARITY: `request_hard_interrupt` (upstream lines 10-36).
pub fn request_hard_interrupt(
    hard_interrupt: Option<InterruptProducer<'_>>,
    interrupt: Option<InterruptProducer<'_>>,
    message: Option<&str>,
) -> bool {
    match (hard_interrupt, interrupt) {
        (Some(hard), _) => {
            hard(message);
            true
        }
        (None, Some(legacy)) => {
            legacy(message);
            true
        }
        (None, None) => false,
    }
}
