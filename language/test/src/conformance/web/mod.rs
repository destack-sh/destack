use std::path::PathBuf;

/// The `web` conformance domain name.
pub const DOMAIN_NAME: &str = "web";

/// Return the Web conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}
