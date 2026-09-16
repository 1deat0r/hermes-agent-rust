//! SMS (Twilio) platform package entry (P4).
//!
//! PARITY: `plugins/platforms/sms/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="sms", …)`.
pub const PLATFORM_NAME: &str = "sms";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers SMS (Twilio) with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "SMS (Twilio)".to_string(),
        required_env: vec![
            "TWILIO_ACCOUNT_SID".to_string(),
            "TWILIO_AUTH_TOKEN".to_string(),
            "TWILIO_PHONE_NUMBER".to_string(),
        ],
        install_hint: "pip install aiohttp".to_string(),
    });
}
