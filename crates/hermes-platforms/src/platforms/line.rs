//! LINE platform package entry (P4).
//!
//! PARITY: `plugins/platforms/line/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="line", …)`.
pub const PLATFORM_NAME: &str = "line";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers LINE with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "LINE".to_string(),
        required_env: vec![
            "LINE_CHANNEL_ACCESS_TOKEN".to_string(),
            "LINE_CHANNEL_SECRET".to_string(),
        ],
        install_hint: "pip install aiohttp".to_string(),
    });
}
