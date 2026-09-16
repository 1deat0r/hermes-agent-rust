//! SimpleX Chat platform package entry (P4).
//!
//! PARITY: `plugins/platforms/simplex/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="simplex", …)`.
pub const PLATFORM_NAME: &str = "simplex";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers SimpleX Chat with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "SimpleX Chat".to_string(),
        required_env: vec![
            "SIMPLEX_WS_URL".to_string(),
        ],
        install_hint: "pip install websockets   # SimpleX adapter requires the websockets package".to_string(),
    });
}
