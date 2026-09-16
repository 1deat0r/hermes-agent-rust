//! DingTalk platform package entry (P4).
//!
//! PARITY: `plugins/platforms/dingtalk/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="dingtalk", …)`.
pub const PLATFORM_NAME: &str = "dingtalk";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers DingTalk with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "DingTalk".to_string(),
        required_env: vec![
            "DINGTALK_CLIENT_ID".to_string(),
            "DINGTALK_CLIENT_SECRET".to_string(),
        ],
        install_hint: "pip install 'dingtalk-stream>=0.20' httpx".to_string(),
    });
}
