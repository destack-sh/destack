use windows_sys::Win32::Security::Cryptography::{
    CERT_SYSTEM_STORE_CURRENT_USER_ID, CERT_SYSTEM_STORE_LOCAL_MACHINE_ID,
    CERT_SYSTEM_STORE_LOCATION_SHIFT,
};

/// Windows root certificate store collection name.
pub(super) const WINDOWS_CERT_STORE_ROOT: &str = "ROOT";

/// Windows intermediate certificate-authority store collection name.
pub(super) const WINDOWS_CERT_STORE_CA: &str = "CA";

/// Windows trusted-people certificate store collection name.
pub(super) const WINDOWS_CERT_STORE_TRUSTED_PEOPLE: &str = "TrustedPeople";

/// Windows store collection used for host-lane certificate imports.
pub(super) const WINDOWS_CERT_STORE_IMPORT: &str = WINDOWS_CERT_STORE_TRUSTED_PEOPLE;

/// Windows store collection probe target for lane availability checks.
pub(super) const WINDOWS_CERT_STORE_PROBE: &str = WINDOWS_CERT_STORE_ROOT;

/// Windows certificate scan order for host store listing.
pub(super) const WINDOWS_CERT_STORE_SCAN_ORDER: [&str; 3] = [
    WINDOWS_CERT_STORE_ROOT,
    WINDOWS_CERT_STORE_CA,
    WINDOWS_CERT_STORE_TRUSTED_PEOPLE,
];

/// Windows certificate delete order for host store lane cleanup.
pub(super) const WINDOWS_CERT_STORE_DELETE_ORDER: [&str; 3] = [
    WINDOWS_CERT_STORE_TRUSTED_PEOPLE,
    WINDOWS_CERT_STORE_CA,
    WINDOWS_CERT_STORE_ROOT,
];

/// Windows user-lane host key snapshot relative path.
pub(super) const WINDOWS_USER_KEYSTORE_RELATIVE_PATH: &str = "Destack/crypto/user-store.keys";

/// Windows machine-lane host key snapshot relative path.
pub(super) const WINDOWS_MACHINE_KEYSTORE_RELATIVE_PATH: &str = "Destack/crypto/machine-store.keys";

/// Windows current-user system-store location value for CertOpenStore flags.
pub(super) const WINDOWS_CERT_STORE_CURRENT_USER: u32 =
    CERT_SYSTEM_STORE_CURRENT_USER_ID << CERT_SYSTEM_STORE_LOCATION_SHIFT;

/// Windows local-machine system-store location value for CertOpenStore flags.
pub(super) const WINDOWS_CERT_STORE_LOCAL_MACHINE: u32 =
    CERT_SYSTEM_STORE_LOCAL_MACHINE_ID << CERT_SYSTEM_STORE_LOCATION_SHIFT;
