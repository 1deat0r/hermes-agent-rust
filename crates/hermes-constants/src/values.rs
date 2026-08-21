//! Plain constant values.
//!
//! PARITY: hermes_constants.py lines 1361–1368.

/// Response ID for partial stream stubs used during error recovery.
pub const PARTIAL_STREAM_STUB_ID: &str = "partial-stream-stub";

/// Finish reason for length-truncated responses.
pub const FINISH_REASON_LENGTH: &str = "length";

/// OpenRouter base URL.
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// OpenRouter models listing URL.
pub const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

/// Vercel AI Gateway base URL.
pub const AI_GATEWAY_BASE_URL: &str = "https://ai-gateway.vercel.sh/v1";
