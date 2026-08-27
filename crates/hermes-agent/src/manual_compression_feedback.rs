//! User-facing summaries for manual compression commands.
//!
//! PARITY: `agent/manual_compression_feedback.py` @ b9aa928 (whole module,
//! lines 1-121).

use hermes_logging::redact_sensitive_text;
use serde_json::{json, Value};

/// The `compression_state` attributes this module reads.
///
/// PARITY: the `getattr(compression_state, "_last_*", ...)` probes in
/// `summarize_manual_compression` (upstream lines 60-105). Rust has no
/// duck-typed attribute access, so the four signals are named fields and each
/// keeps its Python type check: the two flags are `is True` identity tests, the
/// dropped count must be an `int` but NOT a `bool`, and the failure reason must
/// be a non-blank `str`. [`Self::default`] models `compression_state=None`.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionStateSignals {
    /// `_last_compress_aborted` (identity against `True`).
    pub last_compress_aborted: Value,
    /// `_last_summary_fallback_used` (identity against `True`).
    pub last_summary_fallback_used: Value,
    /// `_last_summary_dropped_count` (`int`, excluding `bool`).
    pub last_summary_dropped_count: Value,
    /// `_last_summary_error` (non-blank `str`, otherwise no reason).
    pub last_summary_error: Value,
}

impl Default for CompressionStateSignals {
    /// PARITY: `compression_state is None` — every probe falls to its default.
    fn default() -> Self {
        Self {
            last_compress_aborted: Value::Null,
            last_summary_fallback_used: Value::Null,
            last_summary_dropped_count: Value::Null,
            last_summary_error: Value::Null,
        }
    }
}

impl CompressionStateSignals {
    /// A present compression state with no signals set (upstream's
    /// `getattr(..., False)` / `None` defaults).
    pub fn present() -> Self {
        Self {
            last_compress_aborted: json!(false),
            last_summary_fallback_used: json!(false),
            ..Self::default()
        }
    }

    pub fn with_aborted(mut self, value: bool) -> Self {
        self.last_compress_aborted = json!(value);
        self
    }

    pub fn with_fallback_used(mut self, value: bool) -> Self {
        self.last_summary_fallback_used = json!(value);
        self
    }

    pub fn with_dropped_count(mut self, value: impl Into<Value>) -> Self {
        self.last_summary_dropped_count = value.into();
        self
    }

    pub fn with_summary_error(mut self, value: impl Into<String>) -> Self {
        self.last_summary_error = Value::String(value.into());
        self
    }

    fn aborted(&self) -> bool {
        self.last_compress_aborted == json!(true)
    }

    fn fallback_used(&self) -> bool {
        self.last_summary_fallback_used == json!(true)
    }

    /// `_last_summary_dropped_count` accepting only a real `int`: Python's
    /// `not isinstance(v, int) or isinstance(v, bool)` guard rejects `True`.
    fn dropped_count(&self) -> Option<i64> {
        match &self.last_summary_dropped_count {
            Value::Number(number) => number.as_i64(),
            _ => None,
        }
    }

    /// `_last_summary_error` normalized the way upstream does: a non-string or
    /// blank value means "no reason".
    fn failure_reason(&self) -> Option<String> {
        match &self.last_summary_error {
            Value::String(text) => {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            _ => None,
        }
    }
}

/// The feedback document upstream returns as a dict.
///
/// PARITY: the `return {...}` block (upstream lines 107-114).
#[derive(Debug, Clone, PartialEq)]
pub struct ManualCompressionFeedback {
    pub noop: bool,
    pub aborted: bool,
    pub fallback_used: bool,
    pub headline: String,
    pub token_line: String,
    pub note: Option<String>,
}

/// Python's `f"{value:,}"` group-by-three formatting for a token count.
fn grouped(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if value < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// User-facing text for a manual `/compress` skipped by the compression lock.
///
/// PARITY: `describe_compression_lock_skip` (upstream lines 11-39).
/// `lock_signal` is `agent._compression_skipped_due_to_lock` (or the `holder`
/// carried by the TUI's `CompressionLockHeld`): a descriptive holder string
/// when another compressor CONFIRMED holds the lock, or `None` (upstream
/// `True`/`None`) when acquisition failed without a confirmed holder. The two
/// cases are worded differently on purpose — claiming "already in progress" on
/// an unconfirmed failure misdirects the user when the real problem is a broken
/// lock subsystem.
pub fn describe_compression_lock_skip(lock_signal: Option<&str>) -> String {
    let holder = lock_signal.filter(|holder| !holder.trim().is_empty());
    match holder {
        Some(holder) => format!(
            "⏳ Compression already in progress for this session (holder: \
             {holder}). Please wait for it to finish."
        ),
        None => "⏳ Compression skipped: could not acquire this session's \
                compression lock. Another compression may still be running, or \
                the lock check failed — try again shortly."
            .to_string(),
    }
}

/// Return consistent user-facing feedback for manual compression.
///
/// PARITY: `summarize_manual_compression` (upstream lines 42-120).
/// `state` is `None` for the upstream `compression_state=None` case, where no
/// abort/fallback/error signal can be set.
pub fn summarize_manual_compression(
    before_messages: &[Value],
    after_messages: &[Value],
    before_tokens: i64,
    after_tokens: i64,
    state: Option<&CompressionStateSignals>,
) -> ManualCompressionFeedback {
    let before_count = before_messages.len() as i64;
    let after_count = after_messages.len() as i64;
    let noop = after_messages == before_messages;
    let aborted = state.is_some_and(|state| state.aborted());
    let fallback_used = state.is_some_and(|state| state.fallback_used());
    let failure_reason = state.and_then(CompressionStateSignals::failure_reason);

    let headline = if aborted {
        format!("Compression aborted: {before_count} messages preserved")
    } else if fallback_used {
        format!("Compressed with fallback: {before_count} → {after_count} messages")
    } else if noop {
        format!("No changes from compression: {before_count} messages")
    } else {
        format!("Compressed: {before_count} → {after_count} messages")
    };

    let token_line = if noop && after_tokens == before_tokens {
        format!(
            "Approx request size: ~{} tokens (unchanged)",
            grouped(before_tokens)
        )
    } else {
        format!(
            "Approx request size: ~{} → ~{} tokens",
            grouped(before_tokens),
            grouped(after_tokens)
        )
    };

    let mut note = if aborted {
        Some("Summary generation failed; no messages were removed.".to_string())
    } else if fallback_used {
        let dropped_count = state
            .and_then(CompressionStateSignals::dropped_count)
            .unwrap_or_else(|| (before_count - after_count).max(0));
        Some(format!(
            "Summary generation failed; Hermes used limited fallback context \
             and removed {dropped_count} message(s)."
        ))
    } else if !noop && after_count < before_count && after_tokens > before_tokens {
        Some(
            "Note: fewer messages can still raise this estimate when compression \
             rewrites the transcript into denser summaries."
                .to_string(),
        )
    } else {
        None
    };

    if let (Some(reason), true) = (failure_reason, aborted || fallback_used) {
        // This text crosses a user-facing UI boundary. Never let a disabled
        // global redaction preference expose credentials embedded in provider
        // exception text.
        let safe_reason = redact_sensitive_text(&reason, true, false, false, false);
        note = Some(format!(
            "{} Reason: {safe_reason}",
            // Python renders a missing note as the literal `None`; the branch
            // is unreachable there too, but the string stays identical.
            note.unwrap_or_else(|| "None".to_string())
        ));
    }

    ManualCompressionFeedback {
        noop,
        aborted,
        fallback_used,
        headline,
        token_line,
        note,
    }
}
