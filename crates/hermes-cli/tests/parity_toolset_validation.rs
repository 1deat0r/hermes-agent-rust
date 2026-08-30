// Tier: unit — mirrors `tests/hermes_cli/test_toolset_validation.py` (both
// oracle tests) plus the source branches that oracle leaves unexercised
// (non-mapping input, empty mappings, scalar entry form, skipped names, and
// the no-suggestion arm). The validity predicate is injected exactly like
// upstream's `is_valid_toolset`, so no tool registry is involved.

use hermes_cli::toolset_validation::validate_platform_toolsets;
use serde_json::{json, Value};

// A representative set of real toolset names. `hermes` is deliberately absent —
// that is the corruption #38798 reported (`hermes-cli` rewritten to `hermes`).
fn is_valid(name: &str) -> bool {
    matches!(
        name,
        "hermes-cli" | "hermes-telegram" | "hermes-discord" | "terminal" | "web"
    )
}

fn validate(platform_toolsets: Value) -> Vec<String> {
    validate_platform_toolsets(&platform_toolsets, is_valid)
}

fn zero_toolsets_warning(warnings: &[String]) -> bool {
    warnings.iter().any(|w| w.contains("zero valid toolsets"))
}

// Oracle: test_38798_corruption_warns_and_suggests_correct_name — the exact
// reported shape: cli holds 'hermes' instead of 'hermes-cli'.
#[test]
fn corruption_38798_warns_and_suggests_the_correct_name() {
    let warnings = validate(json!({"cli": ["hermes"]}));

    let unknown: Vec<_> = warnings
        .iter()
        .filter(|w| w.contains("unknown toolset 'hermes'"))
        .collect();
    assert_eq!(unknown.len(), 1);
    // Actionable: points at the valid name the entry should have been.
    assert!(unknown[0].contains("did you mean 'hermes-cli'?"));
    // And the zero-valid-toolsets safety net fires.
    assert!(zero_toolsets_warning(&warnings));
}

// Oracle: test_mixed_valid_and_invalid_flags_only_the_invalid.
#[test]
fn mixed_valid_and_invalid_flags_only_the_invalid() {
    let warnings = validate(json!({"cli": ["hermes-cli"], "discord": ["bogus"]}));

    // One valid entry exists, so no zero-valid warning.
    assert!(!zero_toolsets_warning(&warnings));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("platform 'discord'"));
    assert!(warnings[0].contains("unknown toolset 'bogus'"));
}

// The warning strings are contract, not decoration — assert them byte-exact
// (note the U+2014 em dashes).
#[test]
fn warning_text_is_byte_exact() {
    let warnings = validate(json!({"cli": ["hermes"]}));

    assert!(warnings.contains(
        &("platform 'cli' references unknown toolset 'hermes' — did you mean \
         'hermes-cli'?")
            .to_string()
    ));
    assert!(warnings.contains(
        &("platform_toolsets resolves to zero valid toolsets — the agent will \
         have no tools. Run `hermes tools` to reconfigure.")
            .to_string()
    ));
}

// `raw if isinstance(raw, list) else [raw]`: a bare string entry is the
// same as a one-element list.
#[test]
fn scalar_entry_form_equals_the_list_form() {
    assert_eq!(
        validate(json!({"cli": "hermes"})),
        validate(json!({"cli": ["hermes"]}))
    );
    assert_eq!(validate(json!({"cli": "hermes-cli"})), Vec::<String>::new());
}

// `not isinstance(platform_toolsets, dict) or not platform_toolsets`:
// anything but a non-empty mapping has nothing to validate.
#[test]
fn non_mapping_or_empty_input_yields_no_warnings() {
    for input in [
        json!(null),
        json!("cli"),
        json!(["hermes"]),
        json!(42),
        json!(true),
        json!({}),
    ] {
        assert!(validate(input.clone()).is_empty(), "{input}");
    }
}

// `if not isinstance(name, str) or not name: continue` — skipped names are
// neither validated nor counted as valid, so an all-unusable mapping still
// trips the zero-toolsets safety net.
#[test]
fn non_string_and_empty_names_are_skipped_but_still_count_against_validity() {
    let warnings = validate(json!({"cli": ["", 5, null, ["x"], true]}));

    assert_eq!(warnings.len(), 1);
    assert!(zero_toolsets_warning(&warnings));

    // A skipped name next to a real one changes nothing.
    assert!(validate(json!({"cli": ["hermes-cli", ""]})).is_empty());
}

// The suggestion only appears when `hermes-<platform>` would have been valid
// (the exact #38798 shape); otherwise the warning carries no hint.
#[test]
fn suggestion_requires_hermes_platform_to_be_valid() {
    let warned = validate(json!({"web": ["nope"]}));

    assert_eq!(warned.len(), 2); // unknown + zero-valid
    assert!(warned[0].contains("platform 'web' references unknown toolset 'nope'"));
    assert!(!warned[0].contains("did you mean"));

    // And the hint uses the platform key verbatim.
    let hinted = validate(json!({"telegram": ["nope"]}));
    assert!(hinted[0].contains("did you mean 'hermes-telegram'?"));
}
