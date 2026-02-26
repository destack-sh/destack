/// The static backend marker for this crate.
pub const BACKEND: &str = "rust";

/// Return the crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A client type for Destack.
#[derive(Debug, Clone, Default)]
pub struct Client;

impl Client {
    /// Create a new client.
    pub fn new() -> Self {
        Self
    }

    /// Return the backend marker.
    pub fn backend(&self) -> &'static str {
        BACKEND
    }
}
