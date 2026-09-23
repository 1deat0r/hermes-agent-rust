//! Session-id minting: the ONE place that knows the
//! `YYYYMMDD_HHMMSS_<hex>` shape.
//!
//! PARITY: `hermes_state_ids.py` @ 5d59366 (whole module). stdlib-only on
//! purpose — every surface that creates a session mints through here so
//! lost-and-found salvage classifies schema-less rows by the SAME pattern
//! (a shape change here is a recovery-classification change; keep the
//! prefix stable). Gateway keys are 8 hex chars, portability imports 12
//! (many rows minted in the same second), interactive surfaces share 6
//! (`DEFAULT_HEX_LEN` — the Desktop's candidate regex is pinned to that
//! width).
//!
//! PORT SEAMS: uuid4's 32 hex chars are approximated by two random `u64`s
//! (32 hex chars, same alphabet/width) — only the hex width and prefix are
//! observable contracts; no UUID version nibble is exposed anywhere.

use once_cell::sync::Lazy;
use regex::Regex;

/// The fixed timestamp prefix every session id carries (recovery
/// sentinel: `SESSION_ID_PATTERN` identity with
/// `hermes_cli/session_lost_and_found` is pinned by the upstream ids
/// oracle).
pub static SESSION_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{8}_\d{6}_").expect("session-id prefix"));

/// Interactive surfaces share 6 hex chars.
pub const DEFAULT_HEX_LEN: usize = 6;

/// `<timestamp>_<random hex>` for a fresh session, in the process-local
/// wall clock (upstream `datetime.now()` — naive local time; only the
/// digits are observed).
///
/// PARITY: `new_session_id` (upstream lines 31–37).
pub fn new_session_id(hex_len: usize) -> String {
    let now = chrono::Local::now();
    let stamp = now.format("%Y%m%d_%H%M%S");
    let mut hex = String::with_capacity(32);
    {
        // rand 0.10: free `random()` samples the thread rng.
        hex.push_str(&format!("{:016x}", rand::random::<u64>()));
        hex.push_str(&format!("{:016x}", rand::random::<u64>()));
    }
    let hex = hex.chars().take(hex_len).collect::<String>();
    format!("{stamp}_{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_width_and_prefix() {
        let id = new_session_id(DEFAULT_HEX_LEN);
        assert!(SESSION_ID_PATTERN.is_match(&id), "{id}");
        let re = Regex::new(r"^\d{8}_\d{6}_[0-9a-f]{6}$").unwrap();
        assert!(re.is_match(&id), "{id}");
    }

    #[test]
    fn hex_len_widths() {
        for width in [1usize, 8, 12, 32] {
            let id = new_session_id(width);
            let hex = id.split('_').nth(2).unwrap();
            assert_eq!(hex.len(), width, "{id}");
            assert!(
                hex.chars().all(|c| c.is_ascii_hexdigit()),
                "hex alphabet: {id}"
            );
            assert!(SESSION_ID_PATTERN.is_match(&id), "{id}");
        }
    }

    #[test]
    fn consecutive_mints_differ() {
        let a = new_session_id(12);
        let b = new_session_id(12);
        assert_ne!(a, b, "random suffix must vary: {a} vs {b}");
    }
}
