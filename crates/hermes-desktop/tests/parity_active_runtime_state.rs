//! Parity tests for `apps/desktop/electron/active-runtime-state.ts` @ 5d59366.
//!
//! Oracle: `apps/desktop/electron/active-runtime-state.test.ts` (vitest, 7
//! cases). The module is pure (no Electron imports) — a vertical tracer for
//! the P6 Tauri backend: boot-decision behavior behind a two-function seam.
//!
//! Seam (pre-agreed per tdd skill — the module's entire public interface):
//! `has_valid_bootstrap_marker` + `classify_active_runtime`.
//! Expected values are independent literals from the oracle, never
//! recomputed (mattpocock tdd anti-tautology rule).

use hermes_desktop::active_runtime_state::{
    classify_active_runtime, has_valid_bootstrap_marker, BootstrapMarker, UsabilityReason,
};

fn valid_marker() -> BootstrapMarker {
    BootstrapMarker {
        pinned_commit: Some("1234567890abcdef1234567890abcdef12345678".to_string()),
        schema_version: Some(1),
    }
}

/// Oracle: accepts the current schema with a real-looking commit.
#[test]
fn accepts_current_schema_with_real_commit() {
    assert!(has_valid_bootstrap_marker(&Some(valid_marker()), 1));
}

/// Oracle: rejects missing, wrong-schema, and too-short markers.
#[test]
fn rejects_missing_wrong_schema_and_short_markers() {
    assert!(!has_valid_bootstrap_marker(&None, 1));
    assert!(!has_valid_bootstrap_marker(
        &Some(BootstrapMarker {
            pinned_commit: Some(valid_marker().pinned_commit.unwrap()),
            schema_version: Some(2),
        }),
        1
    ));
    assert!(!has_valid_bootstrap_marker(
        &Some(BootstrapMarker {
            pinned_commit: Some("abc123".to_string()),
            schema_version: Some(1),
        }),
        1
    ));
}

/// Oracle: healthy runtime + missing marker → usable (marker stays false).
#[test]
fn healthy_runtime_without_marker_is_usable() {
    let state = classify_active_runtime(&None, 1, true);
    assert!(!state.has_valid_marker);
    assert!(state.should_use_active_runtime);
    assert_eq!(state.usability_reason, UsabilityReason::Usable);
}

/// Oracle: healthy runtime + stale/malformed marker → usable, marker false.
#[test]
fn healthy_runtime_with_stale_marker_is_usable() {
    let state = classify_active_runtime(
        &Some(BootstrapMarker {
            pinned_commit: Some("abc1234".to_string()),
            schema_version: Some(999),
        }),
        1,
        true,
    );
    assert!(!state.has_valid_marker);
    assert!(state.should_use_active_runtime);
    assert_eq!(state.usability_reason, UsabilityReason::Usable);
}

/// Oracle: unusable runtime + valid marker → unusable (marker stays true).
#[test]
fn unusable_runtime_with_valid_marker_is_unusable() {
    let state = classify_active_runtime(&Some(valid_marker()), 1, false);
    assert!(state.has_valid_marker);
    assert!(!state.should_use_active_runtime);
    assert_eq!(state.usability_reason, UsabilityReason::Unusable);
}

/// Oracle (#60721): CLI-installed runtime, no marker → launches, no bootstrap.
#[test]
fn cli_installed_runtime_launches_instead_of_bootstrapping() {
    let state = classify_active_runtime(&None, 1, true);
    assert!(
        state.should_use_active_runtime,
        "a usable runtime must launch"
    );
    assert!(!state.has_valid_marker, "marker provenance stays honest");
}

/// Oracle (#72166): repair-deleted marker must not strand a healthy install.
#[test]
fn repair_deleted_marker_does_not_strand_healthy_install() {
    assert!(classify_active_runtime(&None, 1, true).should_use_active_runtime);
}

/// Hardening beyond the oracle (code-review finding): the oracle tests the
/// 6-char reject but never the exact-7 boundary (`length < 7`). Both arms
/// pinned here; labeled as mine, not upstream's.
#[test]
fn exact_seven_char_commit_is_the_acceptance_boundary() {
    let at_seven = BootstrapMarker {
        pinned_commit: Some("abc1234".to_string()),
        schema_version: Some(1),
    };
    let below_seven = BootstrapMarker {
        pinned_commit: Some("abc123".to_string()),
        schema_version: Some(1),
    };
    assert!(has_valid_bootstrap_marker(&Some(at_seven), 1));
    assert!(!has_valid_bootstrap_marker(&Some(below_seven), 1));
}

/// Hardening beyond the oracle: length parity is UTF-16 units, not bytes —
/// 4 non-ASCII chars (8 bytes) must still reject.
#[test]
fn length_bound_counts_utf16_units_not_bytes() {
    let wide = BootstrapMarker {
        pinned_commit: Some("éééé".to_string()),
        schema_version: Some(1),
    };
    assert!(!has_valid_bootstrap_marker(&Some(wide), 1));
}
