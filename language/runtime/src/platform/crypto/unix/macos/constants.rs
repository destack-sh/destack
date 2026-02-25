/// Secure enclave key-size in bits.
pub(super) const MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS: i32 = 256;

/// Probe key label.
pub(super) const MACOS_SECURE_ENCLAVE_PROBE_LABEL: &str = "destack.crypto.secure-enclave.probe";

/// Default user-keychain service name for host snapshot bytes.
pub(super) const DEFAULT_MACOS_USER_KEYCHAIN_SERVICE: &str = "dev.symbol.destack.crypto.user.keys";

/// Default user-keychain account name for host snapshot bytes.
pub(super) const DEFAULT_MACOS_USER_KEYCHAIN_ACCOUNT: &str = "default";
