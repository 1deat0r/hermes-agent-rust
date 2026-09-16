//! Feishu / Lark platform package entry (P4).
//!
//! PARITY: `plugins/platforms/feishu/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="feishu", …)`.
pub const PLATFORM_NAME: &str = "feishu";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Feishu / Lark with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Feishu / Lark".to_string(),
        required_env: vec![
            "FEISHU_APP_ID".to_string(),
            "FEISHU_APP_SECRET".to_string(),
        ],
        install_hint: "Run `hermes setup` to install Feishu support.".to_string(),
    });
}
