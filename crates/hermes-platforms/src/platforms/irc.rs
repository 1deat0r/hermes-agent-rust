//! IRC platform package entry (P4).
//!
//! PARITY: `plugins/platforms/irc/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="irc", …)`.
pub const PLATFORM_NAME: &str = "irc";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers IRC with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "IRC".to_string(),
        required_env: vec![
            "IRC_SERVER".to_string(),
            "IRC_CHANNEL".to_string(),
            "IRC_NICKNAME".to_string(),
        ],
        install_hint: "No extra packages needed (stdlib only)".to_string(),
    });
}
