//! Small shared time-formatting helpers for CLI output.
//!
//! PARITY: `hermes_cli/timefmt.py` @ 5d59366 (whole module, incl.
//! `coerce_epoch`).
//!
//! Public home for helpers that used to live as private functions on
//! `hermes_cli.main` — importing that module drags in the whole CLI
//! surface, which lightweight consumers (`hermes status`, dump tooling)
//! should not pay for.
//!
//! The `_at` forms take an explicit now-clock, the equivalent of the
//! upstream tests monkeypatching `_time.time`.

use chrono::TimeZone;

// PARITY: `EPOCH_MIN` / `EPOCH_MAX` (upstream lines 18-19) — corrupt-row
// bounds; a bad row degrades to one `?` cell, never a dead command.
pub const EPOCH_MIN: f64 = 0.0;
pub const EPOCH_MAX: f64 = 4_200_000_000.0;

/// Coercible timestamp input. `datetime` objects have no Rust analog — the
/// caller converts to epoch seconds before calling.
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
/// PARITY: `coerce_epoch` (upstream lines 22-40). `None`/`""` mean "unset"
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
        session_id.map(|s| format!(" on session {s}")).unwrap_or_default()
    );
    None
}

/// Format a timestamp as relative time (e.g., '2h ago', 'yesterday').
///
/// PARITY: `relative_time` (upstream lines 12-24). Python `if not ts` makes
/// `None`, `0`, and other falsy inputs render as `"?"`. The branches use
/// truncating `int(delta / x)` divisions; a future timestamp (negative
/// delta) falls into the `delta < 60` arm and reads "just now". The final
/// branch renders the local-time date (`datetime.fromtimestamp` +
/// `%Y-%m-%d`).
pub fn relative_time(ts: Option<f64>) -> String {
    relative_time_at(ts, current_time())
}

/// Explicit-clock form of [`relative_time`].
pub fn relative_time_at(ts: Option<f64>, now: f64) -> String {
    // PARITY @ 5d59366: `if not ts or (ts := coerce_epoch(...)) is None` —
    // the raw falsy check runs FIRST (None/0.0 → "?", no warning), then the
    // coerce gate (corrupt → warning + "?").
    let raw = match ts {
        Some(ts) if ts != 0.0 => ts,
        _ => return "?".to_string(),
    };
    let ts = match coerce_epoch(EpochInput::Number(raw), None, "last_active") {
        Some(ts) => ts,
        None => return "?".to_string(),
    };
    let delta = now - ts;
    if delta < 60.0 {
        return "just now".to_string();
    }
    if delta < 3600.0 {
        return format!("{}m ago", (delta / 60.0) as i64);
    }
    if delta < 86_400.0 {
        return format!("{}h ago", (delta / 3_600.0) as i64);
    }
    if delta < 172_800.0 {
        return "yesterday".to_string();
    }
    if delta < 604_800.0 {
        return format!("{}d ago", (delta / 86_400.0) as i64);
    }
    // datetime.fromtimestamp(ts).strftime("%Y-%m-%d") — local time.
    let secs = ts.floor();
    let nanos = ((ts - secs) * 1e9) as u32;
    chrono::Local
        .timestamp_opt(secs as i64, nanos)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn current_time() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
