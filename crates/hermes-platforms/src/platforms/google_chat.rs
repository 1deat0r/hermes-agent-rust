//! Google Chat platform package entry (P4).
//!
//! PARITY: `plugins/platforms/google_chat/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="google_chat", …)`.
pub const PLATFORM_NAME: &str = "google_chat";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Google Chat with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Google Chat".to_string(),
        required_env: vec![
            "GOOGLE_CHAT_SERVICE_ACCOUNT_JSON".to_string(),
        ],
        install_hint: "Run `hermes setup` to install Google Chat support.".to_string(),
    });
}
