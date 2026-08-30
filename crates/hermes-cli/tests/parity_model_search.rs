// Tier: unit — mirrors tests/hermes_cli/test_model_search.py for the alias
// surface (the `_filter_indices` half of that oracle lives with
// `hermes_cli.curses_ui` and stays unported).

use hermes_cli::model_search::{model_alias_canonical, model_search_text, MODEL_SEARCH_ALIASES};

#[test]
fn ordinary_ids_keep_their_own_text() {
    assert_eq!(model_search_text("kimi-k2.6"), "kimi-k2.6");
    assert_eq!(model_search_text("glm-5.2"), "glm-5.2");
    assert_eq!(model_search_text(""), "");
}

#[test]
fn a_brand_less_wire_id_gains_the_sibling_tokens() {
    assert_eq!(model_search_text("k3"), "k3 kimi-k3 kimi");
    // Case-insensitive lookup, and the wire id keeps its own spelling because
    // the search haystack must never rewrite what is sent to the provider.
    assert_eq!(model_search_text(" K3 "), "K3 kimi-k3 kimi");
    assert_eq!(MODEL_SEARCH_ALIASES, [("k3", &["kimi-k3", "kimi"][..])]);
}

#[test]
fn canonical_slug_is_the_first_alias_lowercased() {
    assert_eq!(model_alias_canonical("k3"), "kimi-k3");
    assert_eq!(model_alias_canonical(" K3 "), "kimi-k3");
    // Identity for anything without an alias entry, lowercased for dedup use.
    assert_eq!(model_alias_canonical("Kimi-K2.6"), "kimi-k2.6");
    assert_eq!(model_alias_canonical(""), "");
}
