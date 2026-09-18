//! Parity tests for 21 platform package entries @ 5d59366.
//!
//! Oracle: source-as-oracle (3-line re-export roots; a2a's 87-line in-init
//! register; teams' computed hint). Each entry's values are independent
//! literals harvested from its adapter's `register()`. Seam: recording ctx.
//! (Slack covered in parity_slack_entry.rs.)

use hermes_platforms::platforms;
use hermes_platforms::{PlatformRegistration, RecordingCtx};

/// Run an entry against a recording ctx; return the single registration.
fn registered(
    entry: fn(&dyn hermes_platforms::PluginCtx),
    platform_name: &str,
) -> PlatformRegistration {
    let ctx = RecordingCtx::default();
    entry(&ctx);
    let regs = ctx.registrations.lock().unwrap();
    assert_eq!(regs.len(), 1, "{platform_name}");
    assert_eq!(regs[0].name, platform_name);
    regs[0].clone()
}

/// Oracle: `plugins/platforms/a2a/__init__.py` — entry registers `a2a`.
#[test]
fn a2a_entry_registers_with_upstream_values() {
    let reg = registered(platforms::a2a::register, platforms::a2a::PLATFORM_NAME);
    assert_eq!(reg.name, "a2a");
    assert_eq!(reg.label, "A2A");
    assert_eq!(reg.required_env, Vec::<String>::new());
    assert!(reg.install_hint == ("No extra packages needed (stdlib only)"));
}

/// Oracle: `plugins/platforms/buzz/__init__.py` — entry registers `buzz`.
#[test]
fn buzz_entry_registers_with_upstream_values() {
    let reg = registered(platforms::buzz::register, platforms::buzz::PLATFORM_NAME);
    assert_eq!(reg.name, "buzz");
    assert_eq!(reg.label, "Buzz");
    assert_eq!(reg.required_env, ["BUZZ_RELAY_URL", "BUZZ_PRIVATE_KEY"]);
    assert!(reg.install_hint == ("Requires the buzz CLI binary (https://github.com/block/buzz) on PATH or at BUZZ_CLI_PATH"));
}

/// Oracle: `plugins/platforms/dingtalk/__init__.py` — entry registers `dingtalk`.
#[test]
fn dingtalk_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::dingtalk::register,
        platforms::dingtalk::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "dingtalk");
    assert_eq!(reg.label, "DingTalk");
    assert_eq!(
        reg.required_env,
        ["DINGTALK_CLIENT_ID", "DINGTALK_CLIENT_SECRET"]
    );
    assert!(reg.install_hint == ("pip install 'dingtalk-stream>=0.20' httpx"));
}

/// Oracle: `plugins/platforms/discord/__init__.py` — entry registers `discord`.
#[test]
fn discord_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::discord::register,
        platforms::discord::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "discord");
    assert_eq!(reg.label, "Discord");
    assert_eq!(reg.required_env, ["DISCORD_BOT_TOKEN"]);
    assert!(reg.install_hint == ("Run `hermes setup` to install Discord support."));
}

/// Oracle: `plugins/platforms/email/__init__.py` — entry registers `email`.
#[test]
fn email_entry_registers_with_upstream_values() {
    let reg = registered(platforms::email::register, platforms::email::PLATFORM_NAME);
    assert_eq!(reg.name, "email");
    assert_eq!(reg.label, "Email");
    assert_eq!(
        reg.required_env,
        ["EMAIL_ADDRESS", "EMAIL_PASSWORD", "EMAIL_SMTP_HOST"]
    );
    assert!(reg.install_hint == ("Email uses the Python stdlib (smtplib/imaplib) — no extra deps"));
}

/// Oracle: `plugins/platforms/feishu/__init__.py` — entry registers `feishu`.
#[test]
fn feishu_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::feishu::register,
        platforms::feishu::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "feishu");
    assert_eq!(reg.label, "Feishu / Lark");
    assert_eq!(reg.required_env, ["FEISHU_APP_ID", "FEISHU_APP_SECRET"]);
    assert!(reg.install_hint == ("Run `hermes setup` to install Feishu support."));
}

/// Oracle: `plugins/platforms/google_chat/__init__.py` — entry registers `google_chat`.
#[test]
fn google_chat_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::google_chat::register,
        platforms::google_chat::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "google_chat");
    assert_eq!(reg.label, "Google Chat");
    assert_eq!(reg.required_env, ["GOOGLE_CHAT_SERVICE_ACCOUNT_JSON"]);
    assert!(reg.install_hint == ("Run `hermes setup` to install Google Chat support."));
}

/// Oracle: `plugins/platforms/homeassistant/__init__.py` — entry registers `homeassistant`.
#[test]
fn homeassistant_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::homeassistant::register,
        platforms::homeassistant::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "homeassistant");
    assert_eq!(reg.label, "Home Assistant");
    assert_eq!(reg.required_env, ["HASS_TOKEN"]);
    assert!(reg.install_hint == ("pip install aiohttp"));
}

/// Oracle: `plugins/platforms/irc/__init__.py` — entry registers `irc`.
#[test]
fn irc_entry_registers_with_upstream_values() {
    let reg = registered(platforms::irc::register, platforms::irc::PLATFORM_NAME);
    assert_eq!(reg.name, "irc");
    assert_eq!(reg.label, "IRC");
    assert_eq!(
        reg.required_env,
        ["IRC_SERVER", "IRC_CHANNEL", "IRC_NICKNAME"]
    );
    assert!(reg.install_hint == ("No extra packages needed (stdlib only)"));
}

/// Oracle: `plugins/platforms/line/__init__.py` — entry registers `line`.
#[test]
fn line_entry_registers_with_upstream_values() {
    let reg = registered(platforms::line::register, platforms::line::PLATFORM_NAME);
    assert_eq!(reg.name, "line");
    assert_eq!(reg.label, "LINE");
    assert_eq!(
        reg.required_env,
        ["LINE_CHANNEL_ACCESS_TOKEN", "LINE_CHANNEL_SECRET"]
    );
    assert!(reg.install_hint == ("pip install aiohttp"));
}

/// Oracle: `plugins/platforms/matrix/__init__.py` — entry registers `matrix`.
#[test]
fn matrix_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::matrix::register,
        platforms::matrix::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "matrix");
    assert_eq!(reg.label, "Matrix");
    assert_eq!(
        reg.required_env,
        ["MATRIX_HOMESERVER", "MATRIX_ACCESS_TOKEN"]
    );
    assert!(reg.install_hint == ("pip install 'mautrix[encryption]'"));
}

/// Oracle: `plugins/platforms/mattermost/__init__.py` — entry registers `mattermost`.
#[test]
fn mattermost_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::mattermost::register,
        platforms::mattermost::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "mattermost");
    assert_eq!(reg.label, "Mattermost");
    assert_eq!(reg.required_env, ["MATTERMOST_URL", "MATTERMOST_TOKEN"]);
    assert!(reg.install_hint == ("pip install aiohttp"));
}

/// Oracle: `plugins/platforms/ntfy/__init__.py` — entry registers `ntfy`.
#[test]
fn ntfy_entry_registers_with_upstream_values() {
    let reg = registered(platforms::ntfy::register, platforms::ntfy::PLATFORM_NAME);
    assert_eq!(reg.name, "ntfy");
    assert_eq!(reg.label, "ntfy");
    assert_eq!(reg.required_env, ["NTFY_TOPIC"]);
    assert!(reg.install_hint == ("pip install httpx   # already a Hermes dependency"));
}

/// Oracle: `plugins/platforms/photon/__init__.py` — entry registers `photon`.
#[test]
fn photon_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::photon::register,
        platforms::photon::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "photon");
    assert_eq!(reg.label, "iMessage via Photon");
    assert_eq!(
        reg.required_env,
        ["PHOTON_PROJECT_ID", "PHOTON_PROJECT_SECRET"]
    );
    assert!(reg.install_hint == ("Run: hermes photon setup  (logs in via device flow, creates a Spectrum project, links your phone number, installs the spectrum-ts sidecar)."));
}

/// Oracle: `plugins/platforms/raft/__init__.py` — entry registers `raft`.
#[test]
fn raft_entry_registers_with_upstream_values() {
    let reg = registered(platforms::raft::register, platforms::raft::PLATFORM_NAME);
    assert_eq!(reg.name, "raft");
    assert_eq!(reg.label, "Raft");
    assert_eq!(reg.required_env, ["RAFT_PROFILE"]);
    assert!(reg.install_hint == ("Install the Raft CLI from https://raft.build"));
}

/// Oracle: `plugins/platforms/simplex/__init__.py` — entry registers `simplex`.
#[test]
fn simplex_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::simplex::register,
        platforms::simplex::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "simplex");
    assert_eq!(reg.label, "SimpleX Chat");
    assert_eq!(reg.required_env, ["SIMPLEX_WS_URL"]);
    assert!(
        reg.install_hint
            == ("pip install websockets   # SimpleX adapter requires the websockets package")
    );
}

/// Oracle: `plugins/platforms/sms/__init__.py` — entry registers `sms`.
#[test]
fn sms_entry_registers_with_upstream_values() {
    let reg = registered(platforms::sms::register, platforms::sms::PLATFORM_NAME);
    assert_eq!(reg.name, "sms");
    assert_eq!(reg.label, "SMS (Twilio)");
    assert_eq!(
        reg.required_env,
        [
            "TWILIO_ACCOUNT_SID",
            "TWILIO_AUTH_TOKEN",
            "TWILIO_PHONE_NUMBER"
        ]
    );
    assert!(reg.install_hint == ("pip install aiohttp"));
}

/// Oracle: `plugins/platforms/teams/__init__.py` — entry registers `teams`.
#[test]
fn teams_entry_registers_with_upstream_values() {
    let reg = registered(platforms::teams::register, platforms::teams::PLATFORM_NAME);
    // Teams: only the stable prefix is pinned; upstream appends a lazy_deps-computed command.
    assert_eq!(reg.name, "teams");
    assert_eq!(reg.label, "Microsoft Teams");
    assert_eq!(
        reg.required_env,
        ["TEAMS_CLIENT_ID", "TEAMS_CLIENT_SECRET", "TEAMS_TENANT_ID"]
    );
    assert!(reg
        .install_hint
        .starts_with("Teams SDK missing — restart the gateway to auto-install, or run: "));
}

/// Oracle: `plugins/platforms/telegram/__init__.py` — entry registers `telegram`.
#[test]
fn telegram_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::telegram::register,
        platforms::telegram::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "telegram");
    assert_eq!(reg.label, "Telegram");
    assert_eq!(reg.required_env, ["TELEGRAM_BOT_TOKEN"]);
    assert!(reg.install_hint == ("Run `hermes setup` to install Telegram support."));
}

/// Oracle: `plugins/platforms/wecom/__init__.py` — entry registers `wecom`.
#[test]
fn wecom_entry_registers_with_upstream_values() {
    let reg = registered(platforms::wecom::register, platforms::wecom::PLATFORM_NAME);
    assert_eq!(reg.name, "wecom");
    assert_eq!(reg.label, "WeCom (Enterprise WeChat)");
    assert_eq!(reg.required_env, ["WECOM_BOT_ID", "WECOM_SECRET"]);
    assert!(reg.install_hint == ("Run `hermes setup` to install WeCom support."));
}

/// Oracle: `plugins/platforms/whatsapp/__init__.py` — entry registers `whatsapp`.
#[test]
fn whatsapp_entry_registers_with_upstream_values() {
    let reg = registered(
        platforms::whatsapp::register,
        platforms::whatsapp::PLATFORM_NAME,
    );
    assert_eq!(reg.name, "whatsapp");
    assert_eq!(reg.label, "WhatsApp");
    assert_eq!(reg.required_env, ["WHATSAPP_ENABLED"]);
    assert!(
        reg.install_hint
            == ("WhatsApp requires a Node.js bridge — see the WhatsApp messaging docs")
    );
}
