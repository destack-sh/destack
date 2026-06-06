use crate::{BACKEND, capi_abi_version, capi_is_available, version};

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

    /// Return the loaded C ABI version.
    pub fn capi_abi_version(&self) -> u32 {
        capi_abi_version()
    }

    /// Return whether the C ABI surface is available.
    pub fn capi_is_available(&self) -> bool {
        capi_is_available()
    }

    /// Return the crate version.
    pub fn version(&self) -> &'static str {
        version()
    }
}
