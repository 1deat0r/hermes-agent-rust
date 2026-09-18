//! Microsoft Teams platform package entry (P4).
//!
//! PARITY: `plugins/platforms/teams/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! NOTE: DYNAMIC TAIL: upstream appends a lazy_deps-computed pip command; only the stable prefix is pinned.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="teams", …)`.
pub const PLATFORM_NAME: &str = "teams";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Microsoft Teams with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Microsoft Teams".to_string(),
        required_env: vec![
            "TEAMS_CLIENT_ID".to_string(),
            "TEAMS_CLIENT_SECRET".to_string(),
            "TEAMS_TENANT_ID".to_string(),
        ],
        install_hint: "Teams SDK missing — restart the gateway to auto-install, or run: "
            .to_string(),
    });
}
