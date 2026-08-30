//! Picker-only search aliases for model ids.
//!
//! PARITY: `hermes_cli/model_search.py` @ b9aa928 (whole module, lines 1-52).
//!
//! Wire IDs stay unchanged. Some providers report short or brand-less ids (Kimi
//! Coding's flagship is literally `k3`) that users still search for by the
//! familiar `kimi-…` naming of sibling models.
//!
//! CROSS-LANGUAGE SYNC: upstream documents that this table must stay in sync
//! with `ui-tui/src/lib/model-search-text.ts` and
//! `web/src/lib/model-search-text.ts`. Those TypeScript surfaces are part of
//! the UI-parity scope of this port; when they are wired, the alias table moves
//! with them rather than being re-derived.

use std::collections::BTreeMap;

/// PARITY: `_MODEL_SEARCH_ALIASES` (upstream lines 12-14): lowercased wire id →
/// extra tokens appended to the search haystack only.
pub const MODEL_SEARCH_ALIASES: [(&str, &[&str]); 1] = [("k3", &["kimi-k3", "kimi"])];

fn alias_map() -> BTreeMap<&'static str, &'static [&'static str]> {
    MODEL_SEARCH_ALIASES.into_iter().collect()
}

/// Return the canonical public slug for a bare wire-id alias.
///
/// PARITY: `model_alias_canonical` (upstream lines 28-35): identity for ids with
/// no alias entry, lowercased so callers can use the result directly as a dedup
/// key.
pub fn model_alias_canonical(model: &str) -> String {
    let key = model.trim().to_lowercase();
    // The derived map keeps the source's "first alias is the public slug"
    // convention, including the `if aliases` guard that skips empty tuples.
    match MODEL_SEARCH_ALIASES
        .iter()
        .find(|(wire_id, aliases)| *wire_id == key && !aliases.is_empty())
    {
        Some((_, aliases)) => aliases[0].to_ascii_lowercase(),
        None => key,
    }
}

/// Return the haystack used for fuzzy/substring model search.
///
/// PARITY: `model_search_text` (upstream lines 38-50). Never changes the wire id
/// passed to the provider; a blank id returns the input unchanged, matching the
/// source's `return model or ""`.
pub fn model_search_text(model: &str) -> String {
    let mid = model.trim();
    if mid.is_empty() {
        return model.to_string();
    }
    let key = mid.to_lowercase();
    match alias_map().get(key.as_str()) {
        Some(aliases) if !aliases.is_empty() => {
            format!("{mid} {}", aliases.join(" "))
        }
        _ => mid.to_string(),
    }
}
