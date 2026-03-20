use std::path::PathBuf;

/// The `ecmascript` conformance domain name.
pub const DOMAIN_NAME: &str = "ecmascript";

/// Return the ECMAScript conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    super::domain_fixtures_dir(DOMAIN_NAME)
}
