//! Raft platform package entry (P4).
//!
//! PARITY: `plugins/platforms/raft/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="raft", …)`.
pub const PLATFORM_NAME: &str = "raft";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Raft with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Raft".to_string(),
        required_env: vec![
            "RAFT_PROFILE".to_string(),
        ],
        install_hint: "Install the Raft CLI from https://raft.build".to_string(),
    });
}
