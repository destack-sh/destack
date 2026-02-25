use std::path::PathBuf;

use crate::diagnostic::RuntimeError;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::constants::{
    ANDROID_MACHINE_KEYSTORE_ABSOLUTE_PATH, ANDROID_USER_KEYSTORE_RELATIVE_PATH,
    DEFAULT_ANDROID_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_ANDROID_SYSTEM_CERTIFICATE_FILES,
};

/// Return configured or default Android system certificate bundle files.
pub(super) fn configured_system_certificate_files(context: &BindingCallContext) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_files(
        context,
        &DEFAULT_ANDROID_SYSTEM_CERTIFICATE_FILES,
    )
}

/// Return configured or default Android system certificate directories.
pub(super) fn configured_system_certificate_directories(
    context: &BindingCallContext,
) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_directories(
        context,
        &DEFAULT_ANDROID_SYSTEM_CERTIFICATE_DIRECTORIES,
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
        ANDROID_USER_KEYSTORE_RELATIVE_PATH,
        ANDROID_MACHINE_KEYSTORE_ABSOLUTE_PATH,
    )
}

/// Return one ioInvalidData runtime error.
pub(super) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    unix_core::invalid_data(operation, message)
}

/// Return one ioPermissionDenied runtime error.
pub(super) fn permission_denied(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    unix_core::permission_denied(operation, message)
}

/// Return one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    unix_core::not_supported(operation)
}
