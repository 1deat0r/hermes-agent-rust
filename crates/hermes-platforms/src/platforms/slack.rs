//! Slack platform package entry (P4).
//!
//! PARITY: `plugins/platforms/slack/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values below
//! are independent literals from `plugins/platforms/slack/adapter.py`
//! `register()` (lines 6632+): name, label, required env, install hint.
//! The adapter body (`SlackAdapter`, check/setup/standalone fns) is a
//! separate missing row; `register` here records the entry contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="slack", …)`.
pub const PLATFORM_NAME: &str = "slack";

// PARITY: `register(ctx)` entry point (upstream `adapter.py:6632`).
/// Package entry point — registers Slack with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Slack".to_string(),
        required_env: vec!["SLACK_BOT_TOKEN".to_string(), "SLACK_APP_TOKEN".to_string()],
        install_hint: "Run `hermes setup` to install Slack support.".to_string(),
    });
}
