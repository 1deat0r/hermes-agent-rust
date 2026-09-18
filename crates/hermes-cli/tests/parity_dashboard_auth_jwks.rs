//! Parity tests for `classify_jwks_lookup_error`
//! (`hermes_cli/dashboard_auth/base.py` @ 5d59366).
//!
//! Oracle: `tests/plugins/dashboard_auth/test_opaque_bearer_not_unreachable.py`
//! (#94558 — opaque bearers must not surface as "provider unreachable").
//! Only the classifier-unit cases port here; the provider/HTTP cases belong
//! to the unported `plugins/dashboard-auth-nous` + `web_server` slices.
//!
//! The PyJWT exception taxonomy crosses the seam as [`JwksLookupFailure`]:
//! this crate has no PyJWT, so the caller maps its JWT library's error kinds
//! to these variants. Order-sensitivity (DecodeError/PyJWKSetError before
//! their parents) is the caller's mapping contract, documented on the enum
//! and pinned by `subclass_failures_classify_before_parent_kinds`.
//!
//! TDD note (R1-adjudicated S5-B3): tests were written first against the
//! missing items and observed failing at compile time (`E0432/E0433` on
//! `classify_jwks_lookup_error`/`JwksClassify`, session log 2026-09-16);
//! no standalone RED log artifact was retained.

use hermes_cli::dashboard_auth::base::{
    classify_jwks_lookup_error, InvalidCodeError, JwksClassify, JwksLookupFailure, ProviderError,
};

/// PARITY: `test_classifier_maps_transport_failure_to_provider_error`.
#[test]
fn transport_failure_is_provider_error() {
    let JwksClassify::Provider(ProviderError(msg)) =
        classify_jwks_lookup_error(JwksLookupFailure::Connection, "Fail to fetch data")
    else {
        panic!("expected provider error");
    };
    // Oracle pins the prefix verbatim (base.py); a stub with the right
    // variant but wrong text must fail.
    assert!(msg.starts_with("JWKS lookup failed:"), "{msg}");
}

/// PARITY: `test_classifier_maps_unverifiable_token_to_invalid_code`
/// (DecodeError / PyJWKSetError / InvalidTokenError arms).
#[test]
fn unverifiable_tokens_are_invalid_code() {
    for failure in [
        JwksLookupFailure::NotJwt,
        JwksLookupFailure::UnknownKid,
        JwksLookupFailure::InvalidToken,
    ] {
        let JwksClassify::InvalidCode(InvalidCodeError(msg)) =
            classify_jwks_lookup_error(failure, "bad")
        else {
            panic!("expected invalid-code for {failure:?}");
        };
        assert!(
            msg.starts_with("token not verifiable by this provider:"),
            "{msg}"
        );
    }
}

/// R1-adjudicated (S3-B3): `PyJWKError`-shaped key failures are neither
/// client nor token errors — upstream's final arm makes them provider
/// faults, never 401s.
#[test]
fn key_material_failure_is_provider_fault() {
    let err = classify_jwks_lookup_error(JwksLookupFailure::KeyMaterial, "bad kty");
    assert!(matches!(err, JwksClassify::Provider(_)));
}

/// R1-adjudicated (S5-B2): the oracle's order-sensitivity
/// (DecodeError/PyJWKSetError before their parents) is a caller-mapping
/// contract — this test pins the mapping rule the JWT owner must apply:
/// subclass-shaped failures classify as invalid-code even though their
/// parents (`MalformedJwks` ≈ bare `PyJWKClientError`) are provider faults.
#[test]
fn subclass_failures_classify_before_parent_kinds() {
    // A DecodeError-shaped input must map to NotJwt (invalid-code), never
    // to the parent MalformedJwks (provider) bucket.
    let child = classify_jwks_lookup_error(JwksLookupFailure::NotJwt, "Not enough segments");
    let parent =
        classify_jwks_lookup_error(JwksLookupFailure::MalformedJwks, "Not enough segments");
    assert!(matches!(child, JwksClassify::InvalidCode(_)));
    assert!(matches!(parent, JwksClassify::Provider(_)));
}

/// PARITY: `test_classifier_keeps_bare_jwk_client_error_as_provider_fault`.
#[test]
fn malformed_jwks_shape_is_provider_fault() {
    let err = classify_jwks_lookup_error(JwksLookupFailure::MalformedJwks, "weird JWKS shape");
    assert!(matches!(err, JwksClassify::Provider(_)));
}

/// Unknown failures fail closed as provider faults (upstream final arm).
#[test]
fn unknown_failure_is_provider_fault() {
    let err = classify_jwks_lookup_error(JwksLookupFailure::Unknown, "???");
    assert!(matches!(err, JwksClassify::Provider(_)));
}

/// The classified errors carry the source message for diagnostics.
#[test]
fn classified_errors_carry_detail() {
    let JwksClassify::Provider(ProviderError(msg)) =
        classify_jwks_lookup_error(JwksLookupFailure::Connection, "conn refused")
    else {
        panic!("expected provider error");
    };
    assert!(msg.contains("conn refused"), "{msg}");

    let JwksClassify::InvalidCode(InvalidCodeError(msg)) =
        classify_jwks_lookup_error(JwksLookupFailure::NotJwt, "Not enough segments")
    else {
        panic!("expected invalid-code error");
    };
    assert!(msg.contains("Not enough segments"), "{msg}");
}
