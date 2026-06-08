use std::ffi::c_char;

const CAPI_ABI_VERSION: u32 = 1;
const VERSION_CSTR_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

/// Return the C ABI version for the bridge C surface.
#[unsafe(no_mangle)]
pub extern "C" fn destack_capi_abi_version() -> u32 {
    CAPI_ABI_VERSION
}

/// Return the bridge package version as a nul terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn destack_capi_version() -> *const c_char {
    VERSION_CSTR_BYTES.as_ptr().cast::<c_char>()
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use super::*;

    #[test]
    fn test_capi_abi_version_is_non_zero() {
        assert!(destack_capi_abi_version() > 0);
    }

    #[test]
    fn test_capi_version_is_semver() {
        let version = unsafe { CStr::from_ptr(destack_capi_version()) };
        let version = version.to_str().expect("version should be valid utf8");
        let fields = version.split('.').collect::<Vec<_>>();

        assert_eq!(fields.len(), 3);
    }
}
