//! Buzz platform package entry (P4).
//!
//! PARITY: `plugins/platforms/buzz/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="buzz", …)`.
pub const PLATFORM_NAME: &str = "buzz";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Buzz with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Buzz".to_string(),
        required_env: vec![
            "BUZZ_RELAY_URL".to_string(),
            "BUZZ_PRIVATE_KEY".to_string(),
        ],
        install_hint: "Requires the buzz CLI binary (https://github.com/block/buzz) on PATH or at BUZZ_CLI_PATH".to_string(),
    });
}
