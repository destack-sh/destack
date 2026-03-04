/// The static backend marker for this crate.
pub const BACKEND: &str = "rust";

/// Return the crate version.
pub fn version() -> &'static str {
    destack_bridge_core::version()
}

/// Return the loaded capi abi version.
pub fn capi_abi_version() -> u32 {
    destack_bridge_core::capi_abi_version()
}

/// Return whether the capi surface is available.
pub fn capi_is_available() -> bool {
    destack_bridge_core::is_available()
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

    /// Return the loaded capi abi version.
    pub fn capi_abi_version(&self) -> u32 {
        capi_abi_version()
    }

    /// Return whether the capi surface is available.
    pub fn capi_is_available(&self) -> bool {
        capi_is_available()
    }

    /// Return the crate version.
    pub fn version(&self) -> &'static str {
        version()
    }
}
