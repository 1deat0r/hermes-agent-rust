//! Boot-decision for the active Hermes install (P6).
//!
//! PARITY: `apps/desktop/electron/active-runtime-state.ts` @ 5d59366
//! (whole module: `hasValidBootstrapMarker` + `classifyActiveRuntime`).
//!
//! A runtime at `~/.hermes/hermes-agent` can be real and runnable even when
//! Desktop never wrote its first-run bootstrap marker (CLI-installed, or a
//! past build forgot it). Runtime usability is authoritative for "can we
//! launch local Hermes right now?"; the marker is only provenance. A
//! missing/stale marker must never force a healthy install into bootstrap.
//!
//! The TS `BootstrapMarkerLike` (`unknown` fields, runtime type-narrowed)
//! crosses the seam as [`BootstrapMarker`] with `Option` fields: `None`
//! covers `null`/`undefined`/missing/wrong-typed arms. Non-object inputs
//! (strings, numbers, arrays) and wrong-typed fields are the *caller's*
//! coercion to `None` — there is no serde layer in this crate yet, so that
//! arm is a documented seam contract, not an in-Rust branch. Likewise
//! `schema_version` narrows TS `number` to `i64`: the Electron caller only
//! ever passes integer schema versions, so fractional/NaN inputs (which TS
//! `!==` would reject) arrive as `None`.

/// Marker provenance: mirrors the TS shape after its runtime narrowing
/// (`typeof === 'object'`, string `pinnedCommit`, numeric `schemaVersion`).
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapMarker {
    pub pinned_commit: Option<String>,
    pub schema_version: Option<i64>,
}

/// Why a runtime may or may not launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsabilityReason {
    Usable,
    Unusable,
}

/// Boot decision for the active install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRuntimeState {
    pub has_valid_marker: bool,
    pub should_use_active_runtime: bool,
    pub usability_reason: UsabilityReason,
}

/// PARITY: `hasValidBootstrapMarker` — non-object → false; schema mismatch
/// → false; non-string or <7-char `pinnedCommit` → false.
///
/// Length parity note (code-review finding): TS `String.length` counts
/// UTF-16 code units, Rust `str::len()` counts bytes. SHAs are ASCII so
/// the oracle never diverges, but non-ASCII input would — hence
/// `encode_utf16().count()`, matching the spec unit exactly.
pub fn has_valid_bootstrap_marker(marker: &Option<BootstrapMarker>, schema_version: i64) -> bool {
    let Some(m) = marker else {
        return false;
    };
    if m.schema_version != Some(schema_version) {
        return false;
    }
    match &m.pinned_commit {
        Some(commit) if commit.encode_utf16().count() >= 7 => true,
        _ => false,
    }
}

/// PARITY: `classifyActiveRuntime` — usability decides launch; the marker
/// only reports provenance, never overrides a healthy runtime.
pub fn classify_active_runtime(
    marker: &Option<BootstrapMarker>,
    schema_version: i64,
    runtime_usable: bool,
) -> ActiveRuntimeState {
    let has_valid_marker = has_valid_bootstrap_marker(marker, schema_version);
    if !runtime_usable {
        return ActiveRuntimeState {
            has_valid_marker,
            should_use_active_runtime: false,
            usability_reason: UsabilityReason::Unusable,
        };
    }
    ActiveRuntimeState {
        has_valid_marker,
        should_use_active_runtime: true,
        usability_reason: UsabilityReason::Usable,
    }
}
