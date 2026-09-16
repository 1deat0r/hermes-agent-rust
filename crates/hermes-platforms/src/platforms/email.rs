//! Email platform package entry (P4).
//!
//! PARITY: `plugins/platforms/email/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="email", …)`.
pub const PLATFORM_NAME: &str = "email";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers Email with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "Email".to_string(),
        required_env: vec![
            "EMAIL_ADDRESS".to_string(),
            "EMAIL_PASSWORD".to_string(),
            "EMAIL_SMTP_HOST".to_string(),
        ],
        install_hint: "Email uses the Python stdlib (smtplib/imaplib) — no extra deps".to_string(),
    });
}
