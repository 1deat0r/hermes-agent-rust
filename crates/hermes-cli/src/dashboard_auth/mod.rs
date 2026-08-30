//! Dashboard authentication provider framework.
//!
//! PARITY: `hermes_cli/dashboard_auth/__init__.py` @ b9aa928 — PARTIAL:
//! `base.py` (protocol + dataclasses + errors), `registry.py`,
//! `public_paths.py`, `prefix.py`, `audit.py`, and `token_auth.py` (route
//! registry + bearer extraction + provider stacking) are ported;
//! `middleware.py`, `cookies.py`, `routes.py`, `login_page.py`,
//! `native_flow.py`, `ws_tickets.py`, and `audit`'s siblings handling the
//! HTTP layer are PENDING with the web-server surface.
//!
//! The dashboard auth gate engages only when the dashboard binds to a
//! non-loopback host without `--insecure`. In that mode, every request
//! must carry a verified session from one of the registered
//! DashboardAuthProvider plugins. The Nous provider lives in
//! `plugins/dashboard-auth-nous/` and is the default; third parties
//! register their own providers via the plugin hook
//! `ctx.register_dashboard_auth_provider`.

pub mod audit;
pub mod cookies;
pub mod base;
pub mod prefix;
pub mod public_paths;
pub mod registry;
pub mod token_auth;
pub mod ws_tickets;
