use std::path::PathBuf;

/// The `ecma` conformance domain name.
pub const DOMAIN_NAME: &str = "ecma";

/// Return the ECMA conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}

/// Return one ECMA conformance suite root.
pub fn suite_dir(suite: &str) -> PathBuf {
    super::suite_fixtures_dir(DOMAIN_NAME, suite)
}

/// Return one ECMA conformance runnable tests root.
pub fn suite_tests_dir(suite: &str) -> PathBuf {
    super::suite_tests_dir(DOMAIN_NAME, suite)
}
