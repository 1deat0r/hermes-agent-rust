//! Parity tests for `gateway/builtin_hooks/__init__.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (docstring-only root — gap noted). Pins the
//! always-registered contract: the package exists and documents that its
//! hooks are always registered (no hook impls at this pin).

use hermes_gateway::builtin_hooks as _;

/// Package links and carries the always-registered contract.
/// (`use` asserts linkage at compile time; the body pins the doc contract.)
#[test]
fn builtin_hooks_package_surface_is_declared() {
    let doc = include_str!("../src/builtin_hooks.rs");
    assert!(doc.contains("always registered"), "{doc}");
    assert!(doc.contains("5d59366"), "{doc}");
}
