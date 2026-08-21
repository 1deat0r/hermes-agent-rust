//! Truthy/env coercion helpers.
//!
//! PARITY: utils.py lines 19–35 (`TRUTHY_STRINGS`, `is_truthy_value`,
//! `env_var_enabled`) and 527–555 (`env_int`, `env_float`, `env_bool`).

/// Project-wide truthy string set.
pub const TRUTHY_STRINGS: [&str; 4] = ["1", "true", "yes", "on"];

/// Coerce bool-ish values using the shared truthy string set.
///
/// PARITY: utils.py `is_truthy_value` (22–30): None → default; bool → as-is;
/// str → `strip().lower() in TRUTHY_STRINGS`; other → `bool(value)`.
pub fn is_truthy_value(value: Option<&str>, value_bool: Option<bool>, default: bool) -> bool {
    match value {
        Some(s) => {
            let lowered = s.trim().to_lowercase();
            TRUTHY_STRINGS.contains(&lowered.as_str())
        }
        None => match value_bool {
            Some(b) => b,
            None => default,
        },
    }
}

/// Coerce an arbitrary value (bool or string) with the shared truthy set.
///
/// This is the Rust-shaped equivalent of the Python `is_truthy_value` for
/// the only two types callers actually pass (bool and str). Non-str/non-bool
/// Python types fall back to `bool(value)`; in Rust those map to explicit
/// callers passing `value_bool`.
pub fn is_truthy(value: &TruthyValue<'_>, default: bool) -> bool {
    match value {
        TruthyValue::Bool(b) => *b,
        TruthyValue::Str(s) => {
            let lowered = s.trim().to_lowercase();
            TRUTHY_STRINGS.contains(&lowered.as_str())
        }
        TruthyValue::Missing => default,
    }
}

/// Input forms for [`is_truthy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthyValue<'a> {
    Bool(bool),
    Str(&'a str),
    Missing,
}

impl<'a> From<&'a str> for TruthyValue<'a> {
    fn from(v: &'a str) -> Self {
        TruthyValue::Str(v)
    }
}
impl From<bool> for TruthyValue<'_> {
    fn from(v: bool) -> Self {
        TruthyValue::Bool(v)
    }
}

/// Return True when an environment variable is set to a truthy value.
///
/// PARITY: utils.py `env_var_enabled` (33–35) — `os.getenv(name, default)`,
/// default `""`, default-result `False`.
pub fn env_var_enabled(name: &str, default: &str) -> bool {
    let v = std::env::var(name).unwrap_or_else(|_| default.to_string());
    is_truthy(&TruthyValue::Str(&v), false)
}

/// Read an environment variable as an integer, with fallback.
///
/// PARITY: utils.py `env_int` (530–538).
pub fn env_int(key: &str, default: i64) -> i64 {
    let raw = std::env::var(key).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return default;
    }
    raw.parse::<i64>().unwrap_or(default)
}

/// Read an environment variable as a float, with fallback.
///
/// PARITY: utils.py `env_float` (541–549).
pub fn env_float(key: &str, default: f64) -> f64 {
    let raw = std::env::var(key).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return default;
    }
    raw.parse::<f64>().unwrap_or(default)
}

/// Read an environment variable as a boolean.
///
/// PARITY: utils.py `env_bool` (552–554).
pub fn env_bool(key: &str, default: bool) -> bool {
    let v = std::env::var(key).unwrap_or_default();
    is_truthy(&TruthyValue::Str(&v), default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn truthy_strings() {
        for s in TRUTHY_STRINGS {
            assert!(is_truthy(&TruthyValue::Str(s), false), "{}", s);
            assert!(is_truthy(&TruthyValue::Str(&s.to_uppercase()), false), "{}", s);
        }
        for s in ["0", "false", "no", "off", "nope", "  "] {
            assert!(!is_truthy(&TruthyValue::Str(s), false), "{}", s);
        }
    }

    #[test]
    fn truthy_bools_and_defaults() {
        assert!(is_truthy(&TruthyValue::Bool(true), false));
        assert!(!is_truthy(&TruthyValue::Bool(false), true));
        assert!(is_truthy(&TruthyValue::Missing, true));
        assert!(!is_truthy(&TruthyValue::Missing, false));
    }

    #[test]
    fn env_helpers() {
        let _g = ENV_MUTEX.lock().unwrap();
        unsafe { std::env::set_var("HUT_TEST_INT", "42") };
        assert_eq!(env_int("HUT_TEST_INT", 0), 42);
        assert_eq!(env_int("HUT_MISSING", 7), 7);
        unsafe { std::env::set_var("HUT_TEST_BAD", "abc") };
        assert_eq!(env_int("HUT_TEST_BAD", 7), 7);
        unsafe { std::env::set_var("HUT_TEST_FLOAT", "3.5") };
        assert_eq!(env_float("HUT_TEST_FLOAT", 0.0), 3.5);
        unsafe { std::env::set_var("HUT_TEST_BOOL", "TRUE") };
        assert!(env_bool("HUT_TEST_BOOL", false));
        assert!(env_var_enabled("HUT_TEST_BOOL", ""));
        assert!(!env_var_enabled("HUT_MISSING", ""));
        for k in ["HUT_TEST_INT", "HUT_TEST_BAD", "HUT_TEST_FLOAT", "HUT_TEST_BOOL"] {
            unsafe { std::env::remove_var(k) };
        }
    }
}
