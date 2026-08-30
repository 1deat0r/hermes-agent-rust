//! Abstract base + dataclasses + exceptions for dashboard auth providers.
//!
//! PARITY: `hermes_cli/dashboard_auth/base.py` @ b9aa928 (whole module).
//!
//! Lifecycle (see the DashboardAuthProvider docstring upstream):
//!   1. `start_login` — user clicks "Log in with X"; the provider returns a
//!      redirect URL and PKCE/CSRF state for short-lived cookies.
//!   2. Browser bounces through the OAuth IDP to /auth/callback.
//!   3. `complete_login` — exchange code + verifier for a Session.
//!   4. `verify_session` — validate the cookie's access token per request.
//!   5. `refresh_session` — rotate tokens near expiry.
//!   6. `revoke_session` — /auth/logout, best-effort.
//!
//! Failure semantics: `ProviderError` → HTTP 503 (IDP unreachable);
//! `InvalidCodeError` → HTTP 400; `InvalidCredentialsError` → HTTP 401
//! with a deliberately generic detail (never a username oracle);
//! `RefreshExpiredError` → try remaining providers, then force re-login.

use async_trait::async_trait;

/// A verified identity. Returned by `complete_login` and `verify_session`.
///
/// All fields are mandatory. Providers without orgs set `org_id` to "".
/// `access_token` / `refresh_token` are opaque to Hermes.
///
/// PARITY: `Session` (upstream lines 13-26).
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub org_id: String,
    pub provider: String,
    /// Unix seconds; the access_token's `exp` claim.
    pub expires_at: i64,
    pub access_token: String,
    pub refresh_token: String,
}

/// A verified non-interactive (service-to-service) caller — the token
/// analog of [`Session`], attached to the request by the token-auth
/// middleware seam.
///
/// `scopes` — capability strings this principal is authorised for; empty
/// means "unscoped" (the provider vouches but attaches no capability list).
///
/// PARITY: `TokenPrincipal` (upstream lines 29-48).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenPrincipal {
    pub principal: String,
    pub provider: String,
    pub scopes: Vec<String>,
}

/// First leg of the OAuth round trip: `redirect_url` is where the browser
/// must navigate; `cookie_payload` is cookie name → serialised value that
/// the auth route will Set-Cookie. Cookies set here MUST be HttpOnly +
/// Secure (HTTPS) + SameSite=Lax with a TTL ≤ 10 minutes.
///
/// PARITY: `LoginStart` (upstream lines 51-61).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoginStart {
    pub redirect_url: String,
    pub cookie_payload: Vec<(String, String)>,
}

/// PARITY: `ProviderError` (upstream line 64) — IDP unreachable/transient;
/// middleware translates to HTTP 503.
#[derive(Debug, thiserror::Error)]
#[error("provider error: {0}")]
pub struct ProviderError(pub String);

/// PARITY: `InvalidCodeError` (upstream line 68) — OAuth callback
/// code/state failed validation; HTTP 400.
#[derive(Debug, thiserror::Error)]
#[error("invalid code: {0}")]
pub struct InvalidCodeError(pub String);

/// PARITY: `InvalidCredentialsError` (upstream line 72) — username/password
/// rejected; HTTP 401 with a generic detail (no user-vs-password
/// distinction — implementations SHOULD spend constant time on unknown
/// users).
#[derive(Debug, thiserror::Error)]
#[error("invalid credentials: {0}")]
pub struct InvalidCredentialsError(pub String);

/// PARITY: `RefreshExpiredError` (upstream line 76) — the refresh token is
/// dead for this provider; does not prove token ownership in a
/// multi-provider deployment.
#[derive(Debug, thiserror::Error)]
#[error("refresh expired: {0}")]
pub struct RefreshExpiredError(pub String);

/// Outcome of a per-request session verification.
#[derive(Debug, Clone)]
pub enum VerifyOutcome {
    /// Token accepted — the verified identity.
    Verified(Box<Session>),
    /// Expired / unknown token (upstream returns `None`): middleware
    /// triggers refresh or logout.
    Unknown,
}

/// Protocol every dashboard-auth provider plugin implements.
///
/// Capability flags (`supports_password`, `supports_token`,
/// `supports_session`) decide which seams consult the provider; the
/// failure semantics per method are documented on the upstream ABC and
/// preserved here. The `complete_password_login` / `verify_token` defaults
/// fail loudly (`NotImplementedError`) rather than silently accepting
/// credentials — providers that set the flag must override them.
///
/// PARITY: `DashboardAuthProvider` (upstream lines 81-268).
#[async_trait]
pub trait DashboardAuthProvider: Send + Sync {
    /// Lowercase identifier, stable forever.
    fn name(&self) -> &str;
    /// User-facing label on the login page.
    fn display_name(&self) -> &str;
    /// Authenticates via username + password (`complete_password_login`)
    /// rather than (or in addition to) the OAuth redirect flow.
    fn supports_password(&self) -> bool {
        false
    }
    /// Can verify a non-interactive bearer token (`verify_token`).
    fn supports_token(&self) -> bool {
        false
    }
    /// Does the interactive cookie-session flow (login, verify, refresh).
    fn supports_session(&self) -> bool {
        true
    }

    /// Start the OAuth redirect: redirect URL + PKCE/CSRF cookie state.
    async fn start_login(&self, redirect_uri: &str) -> Result<LoginStart, ProviderError>;

    /// Exchange the callback code + verifier for a Session.
    ///
    /// Errors: [`InvalidCodeError`] on bad code/state; [`ProviderError`] if
    /// the IDP is unreachable.
    async fn complete_login(
        &self,
        code: &str,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>>;

    /// Validate the cookie's access token per request. `Ok(None)` on
    /// expiry/unknown token; `Err(ProviderError)` if the IDP is
    /// unreachable (503).
    async fn verify_session(&self, access_token: &str) -> Result<Option<Session>, ProviderError>;

    /// Rotate tokens when the access token is near expiry.
    ///
    /// Errors: [`RefreshExpiredError`] when this provider rejects the
    /// refresh token as dead; [`ProviderError`] on network failure.
    async fn refresh_session(
        &self,
        refresh_token: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>>;

    /// /auth/logout. Best-effort — must not raise.
    async fn revoke_session(&self, refresh_token: &str);

    /// Verify a username/password pair and mint a Session. Only called
    /// when [`DashboardAuthProvider::supports_password`] is True.
    ///
    /// Default fails loudly so an OAuth-only provider that forgets to set
    /// the flag fails loudly rather than silently accepting credentials.
    async fn complete_password_login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        let _ = (username, password);
        Err(Box::new(ProviderError(
            "provider does not support password login (set supports_password = true \
             and override complete_password_login)"
                .to_string(),
        )))
    }

    /// Verify a non-interactive bearer token; return its principal. Only
    /// consulted when [`DashboardAuthProvider::supports_token`] is True,
    /// in registration order, until one returns a principal.
    ///
    /// Contract: `Ok(None)` for a token this provider does NOT recognise —
    /// never an unrecognised-token error, so the seam falls through to the
    /// next provider; `Err(ProviderError)` ONLY for a genuine backing-store
    /// outage.
    ///
    /// Default fails loudly so a provider that sets `supports_token` but
    /// forgets to implement this fails loudly rather than silently
    /// accepting every caller.
    async fn verify_token(&self, token: &str) -> Result<Option<TokenPrincipal>, ProviderError> {
        let _ = token;
        Err(ProviderError(
            "provider does not support token auth (set supports_token = true and \
             override verify_token)"
                .to_string(),
        ))
    }
}

/// Raise a protocol-violation error if `provider` doesn't fully implement
/// the provider protocol (upstream raises `TypeError`; here a descriptive
/// error value).
///
/// Call this in every provider plugin's tests. Returns `Ok(())` on
/// success.
///
/// PARITY: `assert_protocol_compliance` (upstream lines 271-305). Rust has
/// no `__abstractmethods__`/`getattr` introspection: the compile-time
/// trait bound covers the method set, so the runtime check pins the
/// required *attributes* (`name` / `display_name` non-empty) — the check
/// the upstream version can actually enforce dynamically.
pub fn assert_protocol_compliance(provider: &dyn DashboardAuthProvider) -> Result<(), String> {
    for (attr, value) in [
        ("name", provider.name()),
        ("display_name", provider.display_name()),
    ] {
        if value.is_empty() {
            return Err(format!("provider missing or empty attribute: {attr:?}"));
        }
    }
    Ok(())
}
