/// The current C ABI version for bridge native clients.
pub const CAPI_ABI_VERSION: u32 = 1;

/// The static backend marker for the shared bridge core.
pub const BACKEND: &str = "core";

/// Shared bridge metadata.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bridge;

impl Bridge {
    /// Return the backend marker.
    pub const fn backend() -> &'static str {
        BACKEND
    }

    /// Return the bridge package version.
    pub const fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Return the current C ABI version.
    pub const fn capi_abi_version() -> u32 {
        CAPI_ABI_VERSION
    }

    /// Return whether the bridge core is healthy.
    pub const fn is_available() -> bool {
        true
    }
}

/// Return the bridge package version.
pub const fn version() -> &'static str {
    Bridge::version()
}

/// Return the current C ABI version.
pub const fn capi_abi_version() -> u32 {
    Bridge::capi_abi_version()
}

/// Return whether the bridge core is healthy.
pub const fn is_available() -> bool {
    Bridge::is_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_semver() {
        let fields = version().split('.').collect::<Vec<_>>();

        assert_eq!(fields.len(), 3);
        assert!(fields.iter().all(|field| field.parse::<u32>().is_ok()));
    }

    #[test]
    fn test_capi_abi_version_is_non_zero() {
        assert!(capi_abi_version() > 0);
    }

    #[test]
    fn test_is_available_is_true() {
        assert!(is_available());
    }

    #[test]
    fn test_bridge_returns_backend() {
        assert_eq!(Bridge::backend(), BACKEND);
    }
}
