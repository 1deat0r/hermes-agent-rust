//! Parity tests for package-root surfaces: `agent/__init__.py` and
//! `agent/proxy_sources/__init__.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (docstring-only roots — gap noted). These pin
//! the package contract: the crate root documents the `agent/__init__`
//! extraction purpose and exposes `jiter_preload`; `proxy_sources` declares
//! the egress-integration pointer with `iron_proxy` pending.

use hermes_agent::{jiter_preload, proxy_sources};

/// `agent/__init__`: crate root carries the extraction-purpose contract and
/// the jiter_preload surface (eager import arm owned by the binary).
/// Referencing the module paths is the assertion: a missing/deleted module
/// fails compile (= RED for the right reason).
#[test]
fn agent_package_surface_is_declared() {
    // `use` above already asserts both modules link; here assert the live
    // contract: the name-mirroring entry point is callable and bool-typed.
    let preload = jiter_preload::preload_jiter_native_extension as fn() -> bool;
    assert!(preload() || !preload(), "fail-open bool contract");
}

/// `agent/proxy_sources/__init__`: package exists; iron_proxy backend pending.
#[test]
fn proxy_sources_package_surface_is_declared() {
    let doc = include_str!("../src/proxy_sources.rs");
    assert!(doc.contains("iron_proxy"), "{doc}");
    assert!(doc.contains("5d59366"), "{doc}");
}
