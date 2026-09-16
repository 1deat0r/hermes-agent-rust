//! P6 scaffold gate: the crate exists, links, and declares its scope.
//! Replaced module-by-module as Electron surfaces land with real oracles.

use hermes_desktop::scaffold_marker;

#[test]
fn scaffold_declares_pin_and_empty_surface() {
    let marker = scaffold_marker();
    assert!(marker.contains("5d59366"), "{marker}");
    assert!(marker.contains("active_runtime_state ported"), "{marker}");
}
