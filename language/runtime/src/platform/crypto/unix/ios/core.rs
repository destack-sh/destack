use std::path::PathBuf;

use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::constants::{
    DEFAULT_IOS_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_IOS_SYSTEM_CERTIFICATE_FILES,
    IOS_MACHINE_KEYSTORE_ABSOLUTE_PATH, IOS_USER_KEYSTORE_RELATIVE_PATH,
};

/// Return configured or default iOS system certificate bundle files.
pub(super) fn configured_system_certificate_files(context: &BindingCallContext) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_files(context, &DEFAULT_IOS_SYSTEM_CERTIFICATE_FILES)
}

/// Return configured or default iOS system certificate directories.
pub(super) fn configured_system_certificate_directories(
    context: &BindingCallContext,
) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_directories(
        context,
        &DEFAULT_IOS_SYSTEM_CERTIFICATE_DIRECTORIES,
    )
}

/// Return one host key-store snapshot path for one lane when available.
pub(super) fn keystore_path(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    unix_core::keystore_path(
        context,
        kind,
        IOS_USER_KEYSTORE_RELATIVE_PATH,
        IOS_MACHINE_KEYSTORE_ABSOLUTE_PATH,
    )
}

/// Return one ioInvalidData runtime error.
pub(super) use unix_core::invalid_data;

/// Return one ioPermissionDenied runtime error.
pub(super) use unix_core::permission_denied;
