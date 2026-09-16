//! A2A (Agent-to-Agent) platform package entry (P4).
//!
//! PARITY: `plugins/platforms/a2a/__init__.py` @ 5d59366. Unlike the other
//! platform roots, `register` lives IN `__init__.py` (not `.adapter`): it
//! registers the five outbound client tools (fail-open, warning on error)
//! and then the inbound platform adapter (fail-open, warning on error).
//! Values are independent literals from that file. The client-tools half
//! belongs to the `plugins.platforms.a2a.tools` row (missing) — this entry
//! ports the platform-registration half; the fail-open composition is
//! tested when the tools row lands.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="a2a", …)`.
pub const PLATFORM_NAME: &str = "a2a";

// PARITY: `register(ctx)` entry point (`__init__.py:79`).
/// Package entry point — registers A2A with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "A2A".to_string(),
        required_env: vec![],
        install_hint: "No extra packages needed (stdlib only)".to_string(),
    });
}
