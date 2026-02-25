use std::env;
use std::path::{Path, PathBuf};

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;

use super::constants::{
    DEFAULT_IOS_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_IOS_SYSTEM_CERTIFICATE_FILES,
    IOS_MACHINE_KEYSTORE_ABSOLUTE_PATH, IOS_USER_KEYSTORE_RELATIVE_PATH,
};

/// Return configured or default iOS system certificate bundle files.
pub(super) fn configured_system_certificate_files(context: &BindingCallContext) -> Vec<PathBuf> {
    // prefer explicit runtime option overrides
    let configured = &context
        .runtime()
        .module_options
        .crypto
        .system_certificate_files;
    if !configured.is_empty() {
        return configured.clone();
    }

    // fall back to host-default bundle file candidates
    DEFAULT_IOS_SYSTEM_CERTIFICATE_FILES
        .iter()
        .map(PathBuf::from)
        .collect()
}

/// Return configured or default iOS system certificate directories.
pub(super) fn configured_system_certificate_directories(
    context: &BindingCallContext,
) -> Vec<PathBuf> {
    // prefer explicit runtime option overrides
    let configured = &context
        .runtime()
        .module_options
        .crypto
        .system_certificate_directories;
    if !configured.is_empty() {
        return configured.clone();
    }

    // fall back to host-default certificate directory candidates
    DEFAULT_IOS_SYSTEM_CERTIFICATE_DIRECTORIES
        .iter()
        .map(PathBuf::from)
        .collect()
}

/// Return one host key-store snapshot path for one lane when available.
pub(super) fn keystore_path(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    // user-lane path can be configured or derived from HOME
    if kind == CryptoStoreKind::User {
        if let Some(path) = &context
            .runtime()
            .module_options
            .crypto
            .host_store_paths
            .user
        {
            return Some(path.clone());
        }

        let home_directory = env::var_os("HOME")?;
        return Some(Path::new(&home_directory).join(IOS_USER_KEYSTORE_RELATIVE_PATH));
    }

    // machine-lane path can be configured or use the default absolute path
    if kind == CryptoStoreKind::Machine {
        if let Some(path) = &context
            .runtime()
            .module_options
            .crypto
            .host_store_paths
            .machine
        {
            return Some(path.clone());
        }

        return Some(PathBuf::from(IOS_MACHINE_KEYSTORE_ABSOLUTE_PATH));
    }

    None
}

/// Return one ioInvalidData runtime error.
pub(super) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Return one ioPermissionDenied runtime error.
pub(super) fn permission_denied(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Return one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}
