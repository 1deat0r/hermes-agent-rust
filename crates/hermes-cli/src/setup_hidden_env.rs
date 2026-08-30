//! Which platform env vars the setup surfaces hide.
//!
//! PARITY: `hermes_cli/setup_hidden_env.py` @ b9aa928 (whole module, lines
//! 1-57).
//!
//! Every messaging platform ships the same handful of knobs that are either
//! set for the user later or already correct by default. Listing them on a
//! setup form turns "paste your bot token" into a five-field interrogation
//! where none of the answers are discoverable. Hiding them is a
//! *presentation* decision only: the env vars keep working through
//! `hermes config set`, `.env`, and `config.yaml`; the gateway reads them
//! exactly as before. Upstream lives beside `web_server` so the CLI wizard
//! can share it without importing the dashboard's FastAPI surface; this leaf
//! mirrors that placement at the crate layer.

/// Suffix match, so plugin adapters nobody enumerated (IRC, SimpleX, LINE,
/// ntfy) get the same treatment without a code change here.
///
/// PARITY: `SETUP_HIDDEN_ENV_SUFFIXES` (upstream lines 33-47), same order.
///
///   *_HOME_CHANNEL*        the bot offers /sethome on the first chat
///   *_ALLOW_ALL_USERS      defaults off; enabling it is a security decision
///   *_REPLY_TO_MODE        cosmetic threading preference
///   *_REPLY_MODE           same, Mattermost's spelling
///   *_REQUIRE_MENTION      behavior toggle with a sane default
///   *_AUTO_THREAD          same
///   *_FREE_RESPONSE_*      per-channel tuning, done once the bot is in a server
///   *_ALLOWED_CHANNELS     same
///   *_PROXY                only for networks that block the platform
///
/// Allowlists (`*_ALLOWED_USERS`) deliberately stay visible: that IS the
/// decision a new user has to make, and the gateway denies everyone until
/// it's set.
pub const SETUP_HIDDEN_ENV_SUFFIXES: [&str; 13] = [
    "_HOME_CHANNEL",
    "_HOME_CHANNEL_NAME",
    "_HOME_CHANNEL_THREAD_ID",
    "_HOME_ADDRESS",
    "_ALLOW_ALL_USERS",
    "_REPLY_TO_MODE",
    "_REPLY_MODE",
    "_REQUIRE_MENTION",
    "_AUTO_THREAD",
    "_FREE_RESPONSE_CHANNELS",
    "_FREE_RESPONSE_ROOMS",
    "_ALLOWED_CHANNELS",
    "_PROXY",
];

/// True when a var is self-configuring and shouldn't appear in setup forms.
///
/// PARITY: `is_setup_hidden_env` (upstream lines 50-56), i.e.
/// `name.endswith(SETUP_HIDDEN_ENV_SUFFIXES)`. Callers must still keep any
/// var a platform lists as *required* — hiding a required credential would
/// make that platform unconfigurable from the UI.
pub fn is_setup_hidden_env(name: &str) -> bool {
    SETUP_HIDDEN_ENV_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}
