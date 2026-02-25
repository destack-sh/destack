use crate::platform::crypto::CryptoStoreKind;

/// Return whether one host-lane store supports persistent key writes.
pub(crate) fn host_store_supports_key_persistence(kind: CryptoStoreKind) -> bool {
    // user lane persistence is supported across unix hosts
    if kind == CryptoStoreKind::User {
        return true;
    }

    // machine lane persistence requires root on non-macos unix hosts
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if kind == CryptoStoreKind::Machine {
            return unsafe { libc::geteuid() == 0 };
        }
    }

    false
}
