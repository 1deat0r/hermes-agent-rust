//! WeCom (Enterprise WeChat) platform package entry (P4).
//!
//! PARITY: `plugins/platforms/wecom/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="wecom", …)`.
pub const PLATFORM_NAME: &str = "wecom";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers WeCom (Enterprise WeChat) with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "WeCom (Enterprise WeChat)".to_string(),
        required_env: vec!["WECOM_BOT_ID".to_string(), "WECOM_SECRET".to_string()],
        install_hint: "Run `hermes setup` to install WeCom support.".to_string(),
    });
}
