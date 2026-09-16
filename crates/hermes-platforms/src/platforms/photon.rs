//! iMessage via Photon platform package entry (P4).
//!
//! PARITY: `plugins/platforms/photon/__init__.py` @ 5d59366 (whole module):
//! `from .adapter import register` + `__all__ = ["register"]`. Values are
//! independent literals from the adapter's `register()`.
//! Adapter body is a separate missing row; `register` records the entry
//! contract only.

use crate::{PlatformRegistration, PluginCtx};

/// Upstream `register_platform(name="photon", …)`.
pub const PLATFORM_NAME: &str = "photon";

// PARITY: `register(ctx)` entry point.
/// Package entry point — registers iMessage via Photon with the plugin system.
pub fn register(ctx: &dyn PluginCtx) {
    ctx.register_platform(PlatformRegistration {
        name: PLATFORM_NAME.to_string(),
        label: "iMessage via Photon".to_string(),
        required_env: vec![
            "PHOTON_PROJECT_ID".to_string(),
            "PHOTON_PROJECT_SECRET".to_string(),
        ],
        install_hint: "Run: hermes photon setup  (logs in via device flow, creates a Spectrum project, links your phone number, installs the spectrum-ts sidecar).".to_string(),
    });
}
