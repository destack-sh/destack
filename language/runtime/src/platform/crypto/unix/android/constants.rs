/// Android user-lane host key snapshot relative path.
pub(super) const ANDROID_USER_KEYSTORE_RELATIVE_PATH: &str =
    ".local/share/destack/crypto/android-user-store.keys";

/// Android machine-lane host key snapshot absolute path.
pub(super) const ANDROID_MACHINE_KEYSTORE_ABSOLUTE_PATH: &str =
    "/data/misc/destack/crypto/android-machine-store.keys";

/// Default Android system certificate bundle file candidates.
pub(super) const DEFAULT_ANDROID_SYSTEM_CERTIFICATE_FILES: [&str; 1] =
    ["/system/etc/security/cacerts.pem"];

/// Default Android system certificate directory candidates.
pub(super) const DEFAULT_ANDROID_SYSTEM_CERTIFICATE_DIRECTORIES: [&str; 2] = [
    "/system/etc/security/cacerts",
    "/apex/com.android.conscrypt/cacerts",
];
