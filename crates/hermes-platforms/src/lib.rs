//! Messaging-platform plugin entries (P4).
//!
//! PARITY SCOPE: `plugins/__init__.py` ("Hermes plugins package") and
//! `plugins/platforms/*/__init__.py` @ `5d59366` — each package root
//! re-exports its adapter's `register` entry point
//! (`from .adapter import register`, `__all__ = ["register"]`).
//!
//! ## Seam: `PluginCtx`
//!
//! Upstream `register(ctx)` receives the Hermes plugin context and calls
//! `ctx.register_platform(name=…, label=…, adapter_factory=…, …)` with
//! ~15 kwargs. The adapter bodies (factories, check/setup fns) are future
//! rows; this crate ports the *entry contract*: each platform module
//! exposes `PLATFORM_NAME` + `register(ctx: &dyn PluginCtx)`, and the ctx
//! seam records the registration. Tests assert the observable half —
//! "calling the package entry registers this platform" — against a
//! recording ctx, with expected values as independent literals from the
//! upstream adapter (name/label/required-env/install-hint).

use std::sync::Mutex;

/// A platform registration record: the observable subset of upstream's
/// `ctx.register_platform(...)` kwargs that the entry contract pins today.
/// (Adapter factory / check / setup fns ride the adapter rows, not the
/// `__init__` rows.)
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformRegistration {
    pub name: String,
    pub label: String,
    pub required_env: Vec<String>,
    pub install_hint: String,
}

/// Minimal plugin-context seam: what platform entries may call.
pub trait PluginCtx {
    fn register_platform(&self, reg: PlatformRegistration);
}

/// Recording ctx for tests and for hosts that wire registration manually.
#[derive(Debug, Default)]
pub struct RecordingCtx {
    pub registrations: Mutex<Vec<PlatformRegistration>>,
}

impl PluginCtx for RecordingCtx {
    fn register_platform(&self, reg: PlatformRegistration) {
        self.registrations.lock().map(|mut r| r.push(reg)).ok();
    }
}

pub mod platforms;
