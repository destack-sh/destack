use std::path::PathBuf;

/// The `node` conformance domain name.
pub const DOMAIN_NAME: &str = "node";

/// Return the Node conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}
