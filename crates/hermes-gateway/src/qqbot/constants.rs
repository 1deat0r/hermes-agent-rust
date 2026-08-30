//! QQBot package-level constants shared across adapter, onboard, and other
//! modules.
//!
//! PARITY: `gateway/platforms/qqbot/constants.py` @ b9aa928 (whole module).

use once_cell::sync::Lazy;

/// QQBot adapter version — bump on functional changes to the adapter
/// package.
///
/// PARITY: `QQBOT_VERSION` (upstream line 11).
pub const QQBOT_VERSION: &str = "1.1.0";

/// The portal domain is configurable via `QQ_PORTAL_HOST` for corporate
/// proxies or test environments. Default: q.qq.com (production).
///
/// PARITY: `PORTAL_HOST = os.getenv("QQ_PORTAL_HOST", "q.qq.com")` — read
/// once at module import upstream, so a `Lazy` is the faithful translation.
pub static PORTAL_HOST: Lazy<String> =
    Lazy::new(|| std::env::var("QQ_PORTAL_HOST").unwrap_or_else(|_| "q.qq.com".to_string()));

/// PARITY: `API_BASE` (upstream line 17).
pub const API_BASE: &str = "https://api.sgroup.qq.com";
/// PARITY: `TOKEN_URL` (upstream line 18).
pub const TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
/// PARITY: `GATEWAY_URL_PATH` (upstream line 19).
pub const GATEWAY_URL_PATH: &str = "/gateway";

/// QR-code onboard endpoints (on the portal host).
/// PARITY: `ONBOARD_CREATE_PATH` (upstream line 22).
pub const ONBOARD_CREATE_PATH: &str = "/lite/create_bind_task";
/// PARITY: `ONBOARD_POLL_PATH` (upstream line 23).
pub const ONBOARD_POLL_PATH: &str = "/lite/poll_bind_result";
/// PARITY: `QR_URL_TEMPLATE` (upstream lines 24-27).
pub const QR_URL_TEMPLATE: &str =
    "https://q.qq.com/qqbot/openclaw/connect.html?task_id={task_id}&_wv=2&source=hermes";

// Timeouts & retry.

/// PARITY: `DEFAULT_API_TIMEOUT` (upstream line 32).
pub const DEFAULT_API_TIMEOUT: f64 = 30.0;
/// PARITY: `FILE_UPLOAD_TIMEOUT` (upstream line 33).
pub const FILE_UPLOAD_TIMEOUT: f64 = 120.0;
/// PARITY: `CONNECT_TIMEOUT_SECONDS` (upstream line 34).
pub const CONNECT_TIMEOUT_SECONDS: f64 = 20.0;

/// PARITY: `RECONNECT_BACKOFF` (upstream line 36).
pub const RECONNECT_BACKOFF: [i64; 5] = [2, 5, 10, 30, 60];
/// PARITY: `MAX_RECONNECT_ATTEMPTS` (upstream line 37).
pub const MAX_RECONNECT_ATTEMPTS: i64 = 100;
/// PARITY: `RATE_LIMIT_DELAY` (upstream line 38).
pub const RATE_LIMIT_DELAY: i64 = 60;
/// PARITY: `QUICK_DISCONNECT_THRESHOLD` (upstream line 39).
pub const QUICK_DISCONNECT_THRESHOLD: f64 = 5.0;
/// PARITY: `MAX_QUICK_DISCONNECT_COUNT` (upstream line 40).
pub const MAX_QUICK_DISCONNECT_COUNT: i64 = 3;

/// PARITY: `ONBOARD_POLL_INTERVAL` (upstream line 42).
pub const ONBOARD_POLL_INTERVAL: f64 = 2.0;
/// PARITY: `ONBOARD_API_TIMEOUT` (upstream line 43).
pub const ONBOARD_API_TIMEOUT: f64 = 10.0;

// Message limits.

/// PARITY: `MAX_MESSAGE_LENGTH` (upstream line 48).
pub const MAX_MESSAGE_LENGTH: i64 = 4000;
/// PARITY: `DEDUP_WINDOW_SECONDS` (upstream line 49).
pub const DEDUP_WINDOW_SECONDS: i64 = 300;
/// PARITY: `DEDUP_MAX_SIZE` (upstream line 50).
pub const DEDUP_MAX_SIZE: i64 = 1000;

// QQ Bot message types.

/// PARITY: `MSG_TYPE_TEXT` (upstream line 55).
pub const MSG_TYPE_TEXT: i64 = 0;
/// PARITY: `MSG_TYPE_MARKDOWN` (upstream line 56).
pub const MSG_TYPE_MARKDOWN: i64 = 2;
/// PARITY: `MSG_TYPE_MEDIA` (upstream line 57).
pub const MSG_TYPE_MEDIA: i64 = 7;
/// PARITY: `MSG_TYPE_INPUT_NOTIFY` (upstream line 58).
pub const MSG_TYPE_INPUT_NOTIFY: i64 = 6;

// QQ Bot file media types.

/// PARITY: `MEDIA_TYPE_IMAGE` (upstream line 63).
pub const MEDIA_TYPE_IMAGE: i64 = 1;
/// PARITY: `MEDIA_TYPE_VIDEO` (upstream line 64).
pub const MEDIA_TYPE_VIDEO: i64 = 2;
/// PARITY: `MEDIA_TYPE_VOICE` (upstream line 65).
pub const MEDIA_TYPE_VOICE: i64 = 3;
/// PARITY: `MEDIA_TYPE_FILE` (upstream line 66).
pub const MEDIA_TYPE_FILE: i64 = 4;
