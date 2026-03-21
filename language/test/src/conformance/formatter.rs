use std::path::PathBuf;

/// The `formatter` conformance domain name.
pub const DOMAIN_NAME: &str = "formatter";

/// Return the formatter conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}

/// Return one formatter conformance suite root.
pub fn suite_dir(suite: &str) -> PathBuf {
    super::suite_fixtures_dir(DOMAIN_NAME, suite)
}

/// Return one formatter conformance runnable tests root.
pub fn suite_tests_dir(suite: &str) -> PathBuf {
    super::suite_tests_dir(DOMAIN_NAME, suite)
}
