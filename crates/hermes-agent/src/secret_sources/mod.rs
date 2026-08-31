//! External secret source integrations.
//!
//! PARITY: `agent/secret_sources/` @ b9aa928 — PARTIAL: `base.py` (the
//! contract, failure taxonomy, and shared subprocess helper) is ported;
//! `registry.py` (the orchestrator), `bitwarden.py`, `onepassword.py`,
//! `command.py`, and `_cache.py` are PENDING.
//!
//! The bundled set is deliberately closed (policy mirrors memory
//! providers): new third-party secret managers ship as standalone plugin
//! repos that subclass SecretSource and register through
//! PluginContext.register_secret_source().

pub mod base;
pub mod command;
pub mod registry;
