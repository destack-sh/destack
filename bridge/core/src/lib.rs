/// The current C ABI version for bridge-native clients.
pub const CAPI_ABI_VERSION: u32 = 1;

/// The static backend marker for the shared bridge core.
pub const BACKEND: &str = "core";

/// Return the bridge package version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Return the current C ABI version.
pub fn capi_abi_version() -> u32 {
    CAPI_ABI_VERSION
}

/// Return whether the bridge core is healthy.
pub fn is_available() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_semver() {
        let parts = version().split('.').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|part| part.parse::<u32>().is_ok()));
    }

    #[test]
    fn test_capi_abi_version_is_non_zero() {
        assert!(capi_abi_version() > 0);
    }

    #[test]
    fn test_is_available_is_true() {
        assert!(is_available());
    }
}
