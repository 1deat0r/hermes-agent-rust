//! Shared allowlist of `/api/*` paths that bypass dashboard auth.
//!
//! PARITY: `hermes_cli/dashboard_auth/public_paths.py` @ b9aa928 (whole
//! module).
//!
//! Two middlewares enforce dashboard auth and previously kept independent
//! copies of this list — when they drifted, `/api/status` 401'd under one
//! gate and broke the portal's wildcard liveness probe. Centralising the
//! allowlist prevents the next drift. Keep this list minimal — only truly
//! non-sensitive, read-only endpoints belong here. Every entry should be
//! safe to expose to external uptime probes, the logged-out dashboard SPA,
//! and anyone who `curl`s the hostname.

/// PARITY: `PUBLIC_API_PATHS` frozenset (upstream lines 34-50). Membership
/// only, so a slice order is not contract.
pub const PUBLIC_API_PATHS: [&str; 8] = [
    // Minimal process liveness probe for desktop/backend boot handshakes.
    "/api/health",
    // Liveness probe target: version, gateway state, active session count,
    // auth-gate shape. No bodies, no session content, no secrets.
    "/api/status",
    // Read-only config-defaults / schema feeds for the SPA's Config page.
    "/api/config/defaults",
    "/api/config/schema",
    // Read-only model metadata (context windows, etc.).
    "/api/model/info",
    // Read-only theme + plugin manifests for the dashboard skin engine.
    "/api/dashboard/themes",
    "/api/dashboard/plugins",
    // Chronos managed-cron fire webhook (NAS → agent). NOT cookie-gated: it
    // carries its own short-lived NAS-minted JWT (purpose=cron_fire), which
    // the handler verifies as the real auth. The JWT — not this allowlist —
    // is the security boundary.
    "/api/cron/fire",
];

/// PARITY: frozenset membership (`path in PUBLIC_API_PATHS`).
pub fn is_public_api_path(path: &str) -> bool {
    PUBLIC_API_PATHS.contains(&path)
}
