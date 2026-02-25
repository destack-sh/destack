/// iOS user-lane host key snapshot relative path.
pub(super) const IOS_USER_KEYSTORE_RELATIVE_PATH: &str =
    "Library/Application Support/destack/crypto/ios-user-store.keys";

/// iOS machine-lane host key snapshot absolute path.
pub(super) const IOS_MACHINE_KEYSTORE_ABSOLUTE_PATH: &str =
    "/var/db/destack/crypto/ios-machine-store.keys";

/// Default iOS system certificate bundle file candidates.
pub(super) const DEFAULT_IOS_SYSTEM_CERTIFICATE_FILES: [&str; 0] = [];

/// Default iOS system certificate directory candidates.
pub(super) const DEFAULT_IOS_SYSTEM_CERTIFICATE_DIRECTORIES: [&str; 0] = [];

/// iOS secure-enclave key size in bits.
pub(super) const IOS_SECURE_ENCLAVE_KEY_SIZE_BITS: i32 = 256;

/// iOS secure-enclave probe key label.
pub(super) const IOS_SECURE_ENCLAVE_PROBE_LABEL: &str = "destack.crypto.ios.secure-enclave.probe";
