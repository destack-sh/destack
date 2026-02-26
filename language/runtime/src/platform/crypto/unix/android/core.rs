use std::path::PathBuf;

use crate::diagnostic::RuntimeError;
use crate::host::{
    HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND,
    HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, HOST_STATUS_PERMISSION_DENIED,
};
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

/// Resolve one callback runtime identifier for Android host callback routing.
pub(super) fn callback_runtime_id(
    context: &BindingCallContext,
    operation: &'static str,
) -> Result<u64, Box<RuntimeError>> {
    let Some(runtime_id) = context.host().callback_runtime_id() else {
        return Err(not_supported(operation));
    };

    Ok(runtime_id)
}

/// Encode one store kind for Android host callback ABI values.
pub(super) fn host_store_kind(
    kind: CryptoStoreKind,
    _operation: &'static str,
) -> Result<u32, Box<RuntimeError>> {
    let encoded = match kind {
        CryptoStoreKind::System => 1,
        CryptoStoreKind::User => 2,
        CryptoStoreKind::Machine => 3,
        CryptoStoreKind::Provider => 4,
        CryptoStoreKind::Ephemeral => 5,
    };

    Ok(encoded)
}

/// Map one Android host callback status into one runtime result.
pub(super) fn host_status_result(
    status: u32,
    operation: &'static str,
    action: &'static str,
) -> Result<(), Box<RuntimeError>> {
    if status == HOST_STATUS_OK {
        return Ok(());
    }

    if status == HOST_STATUS_NOT_SUPPORTED {
        return Err(not_supported(operation));
    }

    if status == HOST_STATUS_INVALID_ARGUMENT {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} reported one invalid argument"),
        ));
    }

    if status == HOST_STATUS_NOT_FOUND {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} could not resolve one key"),
        ));
    }

    if status == HOST_STATUS_PERMISSION_DENIED {
        return Err(permission_denied(
            operation,
            format!("android host crypto {action} was denied"),
        ));
    }

    if status == HOST_STATUS_FAILED {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} failed"),
        ));
    }

    Err(invalid_data(
        operation,
        format!("android host crypto {action} failed with status code {status}"),
    ))
}
