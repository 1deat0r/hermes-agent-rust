//! Discord platform package entry (P4).
//!
//! PARITY: `plugins/platforms/discord/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="discord", …)`.
pub const PLATFORM_NAME: &str = "discord";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Discord with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Discord".to_string(),
        required_env: vec![
            "DISCORD_BOT_TOKEN".to_string(),
        ],
        install_hint: "Run `hermes setup` to install Discord support.".to_string(),
    });
}
