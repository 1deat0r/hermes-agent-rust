//! Hermes gateway monitoring.
//!
//! PARITY: `agent/monitoring/__init__.py` @ b9aa928.
//!
//! Service health monitoring plus redacted operational diagnostics for the
//! gateway daemon, exported over OTLP to an operator-configured endpoint.
//!
//! `emitter` is the in-process event bus: producers (gateway status hooks,
//! the diagnostic log handler) hand typed events to a fire-and-forget
//! queue, and subscribers (the OTLP streamers) consume them off the hot
//! path. The emitter never blocks or raises into gateway code (the
//! hot-path invariant), and nothing is persisted locally — monitoring is
//! an egress path, not a store.
//!
//! Deliberately out of scope here: run/model/tool trajectory capture,
//! usage analytics, and any content-bearing signal.

pub mod cron_health;
pub mod emitter;
pub mod events;
pub mod redaction;

pub use emitter::{emit, get_emitter};
