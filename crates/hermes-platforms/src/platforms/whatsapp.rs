//! WhatsApp platform package entry (P4).
//!
//! PARITY: `plugins/platforms/whatsapp/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="whatsapp", …)`.
pub const PLATFORM_NAME: &str = "whatsapp";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers WhatsApp with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "WhatsApp".to_string(),
        required_env: vec!["WHATSAPP_ENABLED".to_string()],
        install_hint: "WhatsApp requires a Node.js bridge — see the WhatsApp messaging docs"
            .to_string(),
    });
}
