//! Validation for the `platform_toolsets` config section.
//!
//! PARITY: `hermes_cli/toolset_validation.py` @ b9aa928 (whole module, lines
//! 1-75). Pure, side-effect-free helpers so the logic is unit-testable
//! without importing the tool registry or launching Hermes; the validity
//! predicate is injected (upstream normally passes
//! `toolsets.validate_toolset`).
//!
//! Motivated by #38798: a config migration silently rewrote the valid toolset
//! name `hermes-cli` to the non-existent `hermes`. `resolve_toolset('hermes')`
//! returns an empty list, so every tool silently disappeared with no error,
//! warning, or log entry — the agent degraded to text-only replies and the
//! cause took significant debugging to find. Surfacing invalid toolset names
//! (and the zero-tools end state) loudly turns that silent failure into an
//! actionable one.

use serde_json::Value;

/// Return human-readable warnings for a `platform_toolsets` mapping.
///
/// PARITY: `validate_platform_toolsets` (upstream lines 18-74).
///
/// Two failure modes are reported:
///
/// 1. A toolset name that `is_valid_toolset` rejects — usually a corrupted or
///    renamed entry. When `hermes-<platform>` would have been valid (the
///    exact #38798 shape, where `cli` held `hermes` instead of
///    `hermes-cli`), the warning includes that as a suggestion.
/// 2. The mapping is non-empty but resolves to *zero* valid toolsets, so the
///    agent would start with no tools at all.
///
/// `platform_toolsets` is the raw `platform_toolsets` value from config. Only
/// mapping values carry toolset entries; anything else yields no warnings
/// (nothing to validate). A scalar entry is treated as a one-element list
/// (`raw if isinstance(raw, list) else [raw]`), and non-string or empty names
/// are skipped without being validated or counted as valid. Warnings are
/// emitted in config insertion order (`serde_json`'s workspace
/// `preserve_order` matches Python dict iteration).
pub fn validate_platform_toolsets(
    platform_toolsets: &Value,
    is_valid_toolset: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(entries) = platform_toolsets.as_object() else {
        return warnings;
    };
    if entries.is_empty() {
        return warnings;
    }

    let mut valid_count = 0usize;
    for (platform, raw) in entries {
        let names: Box<dyn Iterator<Item = &Value>> = match raw {
            Value::Array(items) => Box::new(items.iter()),
            other => Box::new(std::iter::once(other)),
        };
        for name in names {
            let Some(name) = name.as_str() else { continue };
            if name.is_empty() {
                continue;
            }
            if is_valid_toolset(name) {
                valid_count += 1;
                continue;
            }
            let suggestion = format!("hermes-{platform}");
            let hint = if is_valid_toolset(&suggestion) {
                format!(" — did you mean '{suggestion}'?")
            } else {
                String::new()
            };
            warnings.push(format!(
                "platform '{platform}' references unknown toolset '{name}'{hint}"
            ));
        }
    }

    if valid_count == 0 {
        warnings.push(
            "platform_toolsets resolves to zero valid toolsets — the agent will \
             have no tools. Run `hermes tools` to reconfigure."
                .to_string(),
        );
    }
    warnings
}
