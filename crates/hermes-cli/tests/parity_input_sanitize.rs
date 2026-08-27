// Tier: unit — mirrors tests/hermes_cli/test_input_sanitize.py for
// `hermes_cli/input_sanitize.py`.

use hermes_cli::input_sanitize::{
    collapse_repeated_input_artifacts, collapse_repeated_input_artifacts_with,
    sanitize_user_prompt_text, strip_leaked_bracketed_paste_wrappers, DEFAULT_MIN_REPEATS,
};

#[test]
fn plain_text_is_unchanged() {
    assert_eq!(
        strip_leaked_bracketed_paste_wrappers("hello world"),
        "hello world"
    );
    assert_eq!(sanitize_user_prompt_text(""), "");
}

#[test]
fn non_wrapper_bracket_forms_in_normal_text_stay() {
    let text = "literal[200~tag and literal[201~tag should stay";
    assert_eq!(strip_leaked_bracketed_paste_wrappers(text), text);
}

#[test]
fn canonical_wrappers_are_stripped_unconditionally() {
    assert_eq!(
        strip_leaked_bracketed_paste_wrappers("a\u{1b}[200~b\u{1b}[201~c"),
        "abc"
    );
    assert_eq!(
        strip_leaked_bracketed_paste_wrappers("a^[[200~b^[[201~c"),
        "abc"
    );
    // A boundary-adjacent degraded wrapper is still removed, keeping the
    // boundary character itself.
    assert_eq!(
        strip_leaked_bracketed_paste_wrappers("> [200~run tests[201~ next"),
        "> run tests next"
    );
}

#[test]
fn issue_62557_corruption_tail_is_dropped() {
    let prefix = "需要时随时叫我。";
    let corrupted = format!("{}[e~[[e{}", prefix, "~[[e".repeat(20));
    assert_eq!(collapse_repeated_input_artifacts(&corrupted), prefix);
    assert_eq!(DEFAULT_MIN_REPEATS, 4);
}

#[test]
fn trailing_punctuation_is_preserved() {
    assert_eq!(collapse_repeated_input_artifacts("wait...."), "wait....");
}

#[test]
fn shorter_runs_than_min_repeats_are_kept() {
    let text = format!("ok{}", "~[[e".repeat(3));
    assert_eq!(collapse_repeated_input_artifacts(&text), text);
    assert_eq!(collapse_repeated_input_artifacts_with(&text, 3), "ok");
    // A zero threshold consumes nothing but still trims a dangling bracket.
    assert_eq!(collapse_repeated_input_artifacts_with("plain", 0), "plain");
}

#[test]
fn sanitizer_combines_wrapper_strip_and_tail_collapse() {
    let corrupted = format!("hello[{}", "~[[e".repeat(8));
    assert_eq!(sanitize_user_prompt_text(&corrupted), "hello");
}
