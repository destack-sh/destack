/// The static backend marker for this crate.
pub const BACKEND: &str = "rust";

/// Return the crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
