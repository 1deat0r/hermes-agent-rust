//! Timestamp coercion shared by every reader/writer of stored epochs.
//!
//! HOMED HERE (not `hermes_cli::timefmt`): the SessionDB layer
//! (`hermes_state_portability` export timings + import `started_at`)
//! needs the same corrupt-row bounds as the CLI, and `hermes-state` sits
//! below `hermes-cli` in the crate graph — a duplicate copy would drift.
//! `hermes_cli::timefmt` re-exports these names, so CLI call sites and
//! its parity tests are unchanged.
//!
//! PARITY: `hermes_cli/timefmt.py` `coerce_epoch` @ 5d59366 (lines 22–40)
//! + `EPOCH_MIN`/`EPOCH_MAX` (lines 18–19).

/// Corrupt-row bounds; a bad row degrades to one `?` cell / missing
/// timing, never a dead command.
pub const EPOCH_MIN: f64 = 0.0;
pub const EPOCH_MAX: f64 = 4_200_000_000.0;

/// Coercible timestamp input. `datetime` objects have no Rust analog — the
/// caller converts to epoch seconds before calling. JSON-cell mapping lives
/// at the call sites that see DB/payload values (keeps this module free of
/// serde deps, mirroring the stdlib-only spirit of `hermes_state_ids`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EpochInput<'a> {
    /// `None` upstream ("unset", silent).
    Empty,
    /// Numbers upstream (`float(value)`).
    Number(f64),
    /// Strings upstream (`float(value)` — whitespace tolerated).
    Text(&'a str),
}

/// A stored timestamp cell as float epoch seconds, or `None` when untrusted.
///
/// PARITY: `coerce_epoch` (upstream lines 22–40). `None`/`""` mean "unset"
/// and stay silent; numbers/numeric strings coerce; anything else,
/// non-finite values, and out-of-range values return `None` after a WARNING
/// naming the session (via the `log` facade) so the corrupt row can be
/// found. Every reader goes through here; every writer refuses bad rows.
pub fn coerce_epoch(value: EpochInput<'_>, session_id: Option<&str>, field: &str) -> Option<f64> {
    let ts = match value {
        EpochInput::Empty => return None,
        EpochInput::Number(n) => n,
        EpochInput::Text(s) if s.is_empty() => return None,
        EpochInput::Text(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
    };
    if EPOCH_MIN <= ts && ts <= EPOCH_MAX {
        return Some(ts);
    }
    // NaN fails the range check above (comparisons are False), matching
    // upstream's `not (MIN <= nan <= MAX)` arm.
    log::warn!(
        "Ignoring corrupt {} {:?}{}",
        field,
        match value {
            EpochInput::Number(n) => format!("{n:?}"),
            EpochInput::Text(s) => format!("{s:?}"),
            EpochInput::Empty => unreachable!(),
        },
        session_id
            .map(|s| format!(" on session {s}"))
            .unwrap_or_default()
    );
    None
}
