//! Parity tests for `tools/skill_provenance.py` @ b9aa928, mirroring
//! upstream `tests/tools/test_skill_provenance.py`.

use hermes_tools::skill_provenance::{
    get_current_write_origin, is_background_review, reset_current_write_origin,
    set_current_write_origin, BACKGROUND_REVIEW,
};

#[test]
fn set_and_get_origin() {
    let token = set_current_write_origin("background_review");
    assert_eq!(get_current_write_origin(), "background_review");
    reset_current_write_origin(token);
    assert!(!is_background_review());
}

#[test]
fn empty_origin_falls_back_to_foreground() {
    let token = set_current_write_origin("");
    // Empty is coerced to "foreground" at the set() boundary.
    assert_eq!(get_current_write_origin(), "foreground");
    reset_current_write_origin(token);
}

#[test]
fn reset_restores_the_prior_value() {
    let outer = set_current_write_origin("foreground");
    let inner = set_current_write_origin(BACKGROUND_REVIEW);
    assert_eq!(get_current_write_origin(), BACKGROUND_REVIEW);
    reset_current_write_origin(inner);
    assert_eq!(get_current_write_origin(), "foreground");
    reset_current_write_origin(outer);
}

#[test]
fn context_isolation_between_copies() {
    // ContextVar scoping: modifications in one copy do not leak out. The
    // Rust stand-in for `contextvars.copy_context().run(...)` is a scoped
    // thread (a fresh context, like a copied Context).
    let original = get_current_write_origin();

    let inside = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                set_current_write_origin(BACKGROUND_REVIEW);
                get_current_write_origin()
            })
            .join()
            .unwrap()
    });
    assert_eq!(inside, BACKGROUND_REVIEW);
    // Parent context unaffected.
    assert_eq!(get_current_write_origin(), original);
}

#[test]
fn default_origin_is_foreground_and_predicate() {
    assert_eq!(get_current_write_origin(), "foreground");
    assert!(!is_background_review());
    let token = set_current_write_origin(BACKGROUND_REVIEW);
    assert!(is_background_review());
    reset_current_write_origin(token);
}
