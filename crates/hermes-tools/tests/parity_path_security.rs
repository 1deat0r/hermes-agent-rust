//! Parity tests for `tools/path_security.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (no dedicated test file — gap noted). Pins
//! `validate_within_dir` (resolve + containment, fail-open message) and the
//! previously-missing `has_traversal_component` pre-check. The PLUGIN-COMPAT
//! lazy `logger` arm is PENDING with `tools.approval` (missing row; Rust has
//! no PEP 562 — the re-export lands with that module).

use hermes_tools::path_security::{has_traversal_component, validate_within_dir};
use std::path::Path;

/// Cheap pre-check fires on a literal `..` component only.
#[test]
fn traversal_precheck_fires_only_on_dotdot() {
    assert!(has_traversal_component("a/../b"));
    assert!(has_traversal_component(".."));
    assert!(has_traversal_component("/x/y/../z"));
    assert!(!has_traversal_component("a/b/c"));
    assert!(!has_traversal_component("a/.../b"));
    assert!(!has_traversal_component("a/..b"));
    assert!(!has_traversal_component(""));
}

/// Containment: inside → None; outside → message mentioning escape.
#[test]
fn validate_within_dir_matches_upstream_contract() {
    let dir = std::env::temp_dir().join("hps_parity_check");
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(validate_within_dir(&dir.join("a.txt"), &dir), None);
    let err = validate_within_dir(std::env::temp_dir().as_path(), &dir).unwrap();
    assert!(err.contains("outside") || err.contains("escapes"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
