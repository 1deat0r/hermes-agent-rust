//! Parity tests for `plugins/platforms/slack/__init__.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (3-line re-export root — gap noted); entry
//! values are independent literals from `adapter.py register()`.
//! Seam: `register` against a recording `PluginCtx`.

use hermes_platforms::platforms::slack::{self, PLATFORM_NAME};
use hermes_platforms::{PlatformRegistration, RecordingCtx};

/// Run the entry against a recording ctx; return the single registration.
fn registered() -> PlatformRegistration {
    let ctx = RecordingCtx::default();
    slack::register(&ctx);
    let regs = ctx.registrations.lock().unwrap();
    assert_eq!(regs.len(), 1);
    regs[0].clone()
}

/// The package entry registers Slack with the upstream name.
#[test]
fn slack_entry_registers_platform_name() {
    let reg = registered();
    assert_eq!(reg.name, "slack");
    assert_eq!(PLATFORM_NAME, "slack");
}

/// Label, required env, and install hint match the upstream entry.
#[test]
fn slack_entry_carries_upstream_registration_values() {
    let reg = registered();
    assert_eq!(reg.label, "Slack");
    assert_eq!(reg.required_env, ["SLACK_BOT_TOKEN", "SLACK_APP_TOKEN"]);
    assert_eq!(
        reg.install_hint,
        "Run `hermes setup` to install Slack support."
    );
}
