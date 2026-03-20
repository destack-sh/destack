use std::path::PathBuf;

/// The `formatter` conformance domain name.
pub const DOMAIN_NAME: &str = "formatter";

/// Return the formatter conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}
