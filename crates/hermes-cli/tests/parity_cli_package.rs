// Tier: unit — mirrors the `hermes_cli/__init__.py` package contract at the
// pinned commit (the upstream `tests/hermes_cli/__init__.py` row is a package
// marker, not an oracle).

use hermes_cli::{RELEASE_DATE, VERSION};

#[test]
fn version_and_release_date_match_the_pinned_package() {
    assert_eq!(VERSION, "0.20.0");
    assert_eq!(RELEASE_DATE, "2026.8.3");
}
