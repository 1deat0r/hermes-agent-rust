//! Agent-layer exception types shared by the transport and streaming paths.
//!
//! PARITY: `agent/errors.py` @ b9aa928 (whole module, lines 1-13).
//!
//! The three upstream classes differ only in their Python base class, which
//! callers use to select a `except` arm; Rust carries that distinction through
//! [`thiserror`] and the module docs below. The message is preserved verbatim
//! because the raise sites compose operator-facing hints into it (for example
//! `agent/ssl_guard.py` line 43 joins the repair hint with a newline).

/// PARITY: `SSLConfigurationError(Exception)` (upstream lines 1-4). Raised when
/// SSL/TLS certificate bundle configuration fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct SSLConfigurationError {
    pub message: String,
}

/// PARITY: `EmptyStreamError(RuntimeError)` (upstream lines 7-11). Raised when
/// a provider closes a stream without yielding a response.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct EmptyStreamError {
    pub message: String,
}

/// PARITY: `MoAPresetNotFoundError(ValueError)` (upstream lines 14-16). Raised
/// when a persisted MoA preset no longer exists in config.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct MoAPresetNotFoundError {
    pub message: String,
}

macro_rules! newtype_error {
    ($ty:ty) => {
        impl $ty {
            /// Wrap an operator-facing message, matching the single-argument
            /// `raise ErrorType(msg)` raise sites upstream.
            pub fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }

            /// The message a Python `str(exc)` would render.
            pub fn message(&self) -> &str {
                &self.message
            }
        }
    };
}

newtype_error!(SSLConfigurationError);
newtype_error!(EmptyStreamError);
newtype_error!(MoAPresetNotFoundError);
