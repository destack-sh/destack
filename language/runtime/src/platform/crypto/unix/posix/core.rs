use std::path::PathBuf;

use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::constants::{
    DEFAULT_UNIX_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_UNIX_SYSTEM_CERTIFICATE_FILES,
    UNIX_MACHINE_KEYSTORE_ABSOLUTE_PATH, UNIX_USER_KEYSTORE_RELATIVE_PATH,
};

/// Return configured or default Unix system certificate bundle files.
pub(super) fn configured_system_certificate_files(binding: &BindingCallContext) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_files(binding, &DEFAULT_UNIX_SYSTEM_CERTIFICATE_FILES)
}

/// Return configured or default Unix system certificate directories.
pub(super) fn configured_system_certificate_directories(
    binding: &BindingCallContext,
) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_directories(
        binding,
        &DEFAULT_UNIX_SYSTEM_CERTIFICATE_DIRECTORIES,
    )
}

/// Return one host key-store snapshot path for one lane when available.
pub(super) fn keystore_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    unix_core::keystore_path(
        binding,
        kind,
        UNIX_USER_KEYSTORE_RELATIVE_PATH,
        UNIX_MACHINE_KEYSTORE_ABSOLUTE_PATH,
    )
}
