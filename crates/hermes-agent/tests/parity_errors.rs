// Tier: unit — mirrors the raise-site contract of `agent/errors.py`
// (upstream has no dedicated test module for it; the classes are asserted
// through `tests/agent/test_ssl_ca_guard.py`,
// `tests/run_agent/test_streaming.py`, and `tests/hermes_cli/test_moa_config.py`).

use hermes_agent::errors::{EmptyStreamError, MoAPresetNotFoundError, SSLConfigurationError};

#[test]
fn error_types_render_their_python_message() {
    let ssl = SSLConfigurationError::new(
        "CA bundle is unreadable\nSet SSL_CERT_FILE to a readable bundle.",
    );
    assert_eq!(
        ssl.to_string(),
        "CA bundle is unreadable\nSet SSL_CERT_FILE to a readable bundle."
    );

    let empty = EmptyStreamError::new("provider closed the stream without a response");
    assert_eq!(
        empty.to_string(),
        "provider closed the stream without a response"
    );

    let preset = MoAPresetNotFoundError::new("preset 'fast' no longer exists in config");
    assert_eq!(
        preset.to_string(),
        "preset 'fast' no longer exists in config"
    );
}

#[test]
fn error_types_are_distinct_and_downcastable() {
    let boxed: Box<dyn std::error::Error> = Box::new(MoAPresetNotFoundError::new("missing preset"));
    assert!(boxed.downcast_ref::<MoAPresetNotFoundError>().is_some());
    assert!(boxed.downcast_ref::<EmptyStreamError>().is_none());
    assert!(boxed.downcast_ref::<SSLConfigurationError>().is_none());
}
