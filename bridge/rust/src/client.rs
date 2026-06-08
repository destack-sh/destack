use crate::{BACKEND, version};

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

    /// Return the crate version.
    pub fn version(&self) -> &'static str {
        version()
    }
}
