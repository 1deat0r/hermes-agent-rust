//! Parity tests for `agent/bounded_response.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle. The httpx
//! response is abstracted as a blocking chunk iterator.

use std::time::{Duration, Instant};

use hermes_agent::bounded_response::{
    read_error_body_or_default, read_streaming_error_body, DEFAULT_ERROR_BODY_MAX_BYTES,
    DEFAULT_ERROR_BODY_TIMEOUT_S,
};

#[test]
fn defaults_match_upstream() {
    assert_eq!(DEFAULT_ERROR_BODY_MAX_BYTES, 64 * 1024);
    assert_eq!(DEFAULT_ERROR_BODY_TIMEOUT_S, 10.0);
}

#[test]
fn complete_body_is_returned_verbatim() {
    let body = b"{\"error\": \"bad request\"}".to_vec();
    let iter = Box::new(vec![body].into_iter());
    let text = read_streaming_error_body(iter, DEFAULT_ERROR_BODY_MAX_BYTES, 5.0);
    assert_eq!(text, "{\"error\": \"bad request\"}");
}

#[test]
fn byte_cap_truncates_oversize_bodies() {
    // A body far larger than a tiny cap: capped at the cap, no more.
    let big = vec![b'a'; 5000];
    let iter = Box::new(vec![big].into_iter());
    let text = read_streaming_error_body(iter, 1024, 5.0);
    assert_eq!(text.len(), 1024);
    assert!(text.chars().all(|c| c == 'a'));
}

#[test]
fn many_chunks_accumulate_up_to_the_cap() {
    let chunk = vec![b'x'; 600];
    let iter = Box::new(std::iter::repeat(chunk).take(10));
    let text = read_streaming_error_body(iter, 1024, 5.0);
    assert_eq!(text.len(), 1024, "capped mid-stream at exactly the cap");
}

#[test]
fn stalling_body_hits_the_hard_deadline_and_keeps_partials() {
    // A server that opens the body, sends a first chunk, then stalls
    // forever: the wall-clock deadline abandons the read but keeps the
    // partial bytes.
    let iter = Box::new(
        std::iter::once(b"partial ".to_vec()).chain(std::iter::from_fn(|| {
            std::thread::sleep(Duration::from_secs(60));
            Some(vec![b'y'; 10])
        })),
    );
    let started = Instant::now();
    let text = read_streaming_error_body(iter, DEFAULT_ERROR_BODY_MAX_BYTES, 1.0);
    let elapsed = started.elapsed();
    assert!(text.starts_with("partial "), "{text:?}");
    assert!(
        elapsed < Duration::from_secs(30),
        "hard deadline must bound the read, took {elapsed:?}"
    );
}

#[test]
fn empty_body_yields_empty_and_none_variants() {
    let iter = Box::new(Vec::<Vec<u8>>::new().into_iter());
    assert_eq!(read_streaming_error_body(iter, 1024, 1.0), "");
    let iter = Box::new(Vec::<Vec<u8>>::new().into_iter());
    assert_eq!(read_error_body_or_default(iter, 1024, 1.0), None);
}

#[test]
fn or_default_distinguishes_no_body_from_empty_string() {
    let iter = Box::new(vec![b"data".to_vec()].into_iter());
    assert_eq!(
        read_error_body_or_default(iter, 1024, 1.0).as_deref(),
        Some("data")
    );
}

#[test]
fn invalid_utf8_is_replaced_not_rejected() {
    // UTF-8 errors="replace": an invalid byte becomes U+FFFD.
    let iter = Box::new(vec![vec![0xff, b'o', b'k']].into_iter());
    let text = read_streaming_error_body(iter, 1024, 1.0);
    assert!(text.contains('\u{fffd}'), "{text:?}");
    assert!(text.contains("ok"));
}
