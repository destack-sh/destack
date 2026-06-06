/// The static backend marker for this crate.
pub const BACKEND: &str = "rust";

/// Return the crate version.
pub fn version() -> &'static str {
    destack_bridge_core::version()
}

/// Return the loaded C ABI version.
pub fn capi_abi_version() -> u32 {
    destack_bridge_core::capi_abi_version()
}

/// Return whether the C ABI surface is available.
pub fn capi_is_available() -> bool {
    destack_bridge_core::is_available()
}
