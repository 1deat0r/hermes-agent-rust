//! Hermes Agent Desktop backend port (P6).
//!
//! PARITY SCOPE: `apps/desktop` (Electron 40 main process + React renderer,
//! desktop `0.17.3`, Linux AppImage/deb/rpm via electron-builder),
//! `apps/shared` (`@hermes/shared` contract lib), and the Tauri
//! `apps/bootstrap-installer` (`0.21.1`) — upstream @ `5d59366`.
//!
//! ## Strategy (board-approved R1, 2026-09-16)
//!
//! The Electron main-process TypeScript modules (`apps/desktop/electron/*`,
//! ~340 files: backend lifecycle, window management, IPC, updater, …) port
//! to Rust backend commands via **Tauri** — upstream's own precedent is the
//! Tauri `apps/bootstrap-installer` (`src-tauri/` with real Rust: bootstrap,
//! install-script, powershell, paths, events, update). The React renderer
//! (`apps/desktop/src/*`, ~2,100 files) strategy is decided **per-module**:
//! retain webview + typed command bridge where the UI is presentational,
//! port state-machine logic to Rust where behavior is observable.
//!
//! Linux packaging parity (AppImage/deb/rpm, `build.linux` in
//! `apps/desktop/package.json`) rides the Tauri bundler once the backend
//! surface lands.
//!
//! ## Status
//!
//! Scaffold only: no `ts:` row is `done` yet. Each Electron module ports
//! bottom-up (backend-child/lifecycle first) with upstream-derived parity
//! tests (`apps/desktop/electron/*.test.ts` are the oracles) before any
//! renderer work. One module per commit, TDD, tests-first.

pub mod active_runtime_state;

/// P6 scaffold marker: the crate links and its contract is documented.
/// Every ported Electron module registers its surface here as it lands.
pub fn scaffold_marker() -> &'static str {
    "hermes-desktop P6 scaffold (5d59366): active_runtime_state ported"
}
