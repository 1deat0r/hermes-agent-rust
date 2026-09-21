//! Dashboard authentication provider framework.
//!
//! PARITY: `hermes_cli/dashboard_auth/__init__.py` @ 5d59366 (whole
//! module, 20 lines) + every sibling: `base`, `registry`,
//! `public_paths`, `prefix`, `audit`, `token_auth`, `ws_tickets`,
//! `native_flow`, `cookies`, `request_utils`, `refresh_singleflight`,
//! `middleware` (decision tables + gate flow), `route_logic` (the pure
//! helpers of `routes.py`), `login_page` (escaping + render decisions).
//! The FastAPI route handlers and HTML templates ride with the
//! web-server surface; every portable decision is ported.
//!
//! The dashboard auth gate engages only when the dashboard binds to a
//! non-loopback host without `--insecure`. In that mode, every request
//! must carry a verified session from one of the registered
//! DashboardAuthProvider plugins. The Nous provider lives in
//! `plugins/dashboard-auth-nous/` and is the default; third parties
//! register their own providers via the plugin hook
//! `ctx.register_dashboard_auth_provider`.

pub mod audit;
pub mod base;
pub mod cookies;
pub mod login_page;
pub mod middleware;
pub mod native_flow;
pub mod prefix;
pub mod public_paths;
pub mod refresh_singleflight;
pub mod registry;
pub mod request_utils;
pub mod route_logic;
pub mod token_auth;
pub mod ws_tickets;
