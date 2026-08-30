// Tier: unit — mirrors `tests/hermes_cli/test_setup_hidden_env.py`'s
// `is_setup_hidden_env` oracle (the parametrized knob list and the
// plugin-platform suffix sweep). The card, wizard, and gateway halves of the
// upstream file drive `web_server`, the setup wizard, and the gateway env
// reader — unported surfaces; the predicate itself is the portable contract.

use hermes_cli::setup_hidden_env::{is_setup_hidden_env, SETUP_HIDDEN_ENV_SUFFIXES};

// Oracle: TestIsSetupHiddenEnv::test_self_configuring_knobs_are_hidden.
#[test]
fn self_configuring_knobs_are_hidden() {
    for key in [
        "DISCORD_HOME_CHANNEL",
        "DISCORD_HOME_CHANNEL_NAME",
        "DISCORD_ALLOW_ALL_USERS",
        "DISCORD_REPLY_TO_MODE",
        "MATTERMOST_REPLY_MODE",
        "TELEGRAM_PROXY",
    ] {
        assert!(is_setup_hidden_env(key), "{key}");
    }
}

// Oracle: test_applies_to_plugin_platforms_nobody_enumerated — suffix
// matching is the point; IRC/SimpleX/ntfy/LINE get this for free.
#[test]
fn applies_to_plugin_platforms_nobody_enumerated() {
    for key in [
        "IRC_ALLOW_ALL_USERS",
        "SIMPLEX_HOME_CHANNEL",
        "NTFY_HOME_CHANNEL_NAME",
        "LINE_ALLOW_ALL_USERS",
    ] {
        assert!(is_setup_hidden_env(key), "{key}");
    }
}

// The inverse of the card oracle: credentials and allowlists must stay
// visible (test_discord_card_asks_for_token_and_allowlist_only expects
// exactly DISCORD_BOT_TOKEN + DISCORD_ALLOWED_USERS on the card;
// test_required_token_still_gates_setup expects MATTERMOST_TOKEN asked).
#[test]
fn credentials_and_allowlists_stay_visible() {
    for key in [
        "DISCORD_BOT_TOKEN",
        "DISCORD_ALLOWED_USERS",
        "MATTERMOST_TOKEN",
    ] {
        assert!(!is_setup_hidden_env(key), "{key}");
    }
}

// `str.endswith` matches whole suffixes: a name that merely contains one, or
// equals it without the leading underscore, stays visible.
#[test]
fn suffix_matching_respects_the_leading_underscore() {
    assert!(!is_setup_hidden_env("HOME_CHANNEL"));
    assert!(!is_setup_hidden_env("MY_PROXY_PREFIX"));
    assert!(!is_setup_hidden_env(""));
    // Both reply-mode spellings from the suffix table hit.
    assert!(is_setup_hidden_env("DISCORD_REPLY_TO_MODE"));
    assert!(is_setup_hidden_env("MATTERMOST_REPLY_MODE"));
    // Each suffix in the table hides at least its canonical example.
    for suffix in SETUP_HIDDEN_ENV_SUFFIXES {
        assert!(is_setup_hidden_env(&format!("EXAMPLE{suffix}")), "{suffix}");
    }
}
