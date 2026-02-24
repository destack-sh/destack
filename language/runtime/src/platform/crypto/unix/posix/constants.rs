/// Store-open operation name.
pub(super) const STORE_OPEN_OPERATION: &str = "destack.crypto.store.open";

/// Unix user-lane host key snapshot relative path.
pub(super) const UNIX_USER_KEYSTORE_RELATIVE_PATH: &str =
    ".local/share/destack/crypto/user-store.keys";

/// Unix machine-lane host key snapshot absolute path.
pub(super) const UNIX_MACHINE_KEYSTORE_ABSOLUTE_PATH: &str =
    "/var/lib/destack/crypto/machine-store.keys";

/// Default Linux and Unix-like system certificate bundle file candidates.
pub(super) const DEFAULT_UNIX_SYSTEM_CERTIFICATE_FILES: [&str; 4] = [
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

/// Default Linux and Unix-like certificate directory candidates.
pub(super) const DEFAULT_UNIX_SYSTEM_CERTIFICATE_DIRECTORIES: [&str; 4] = [
    "/etc/ssl/certs",
    "/etc/pki/ca-trust/source/anchors",
    "/usr/local/share/ca-certificates",
    "/etc/openssl/certs",
];
