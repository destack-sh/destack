use std::path::PathBuf;

use crate::diagnostic::RuntimeError;
use crate::host::HostStatus;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::constants::{
    ANDROID_MACHINE_KEYSTORE_ABSOLUTE_PATH, ANDROID_USER_KEYSTORE_RELATIVE_PATH,
    DEFAULT_ANDROID_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_ANDROID_SYSTEM_CERTIFICATE_FILES,
};

/// Return configured or default Android system certificate bundle files.
pub(super) fn configured_system_certificate_files(binding: &BindingCallContext) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_files(
        binding,
        &DEFAULT_ANDROID_SYSTEM_CERTIFICATE_FILES,
    )
}

/// Return configured or default Android system certificate directories.
pub(super) fn configured_system_certificate_directories(
    binding: &BindingCallContext,
) -> Vec<PathBuf> {
    unix_core::configured_system_certificate_directories(
        binding,
        &DEFAULT_ANDROID_SYSTEM_CERTIFICATE_DIRECTORIES,
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
        ANDROID_USER_KEYSTORE_RELATIVE_PATH,
        ANDROID_MACHINE_KEYSTORE_ABSOLUTE_PATH,
    )
}

/// Return one ioInvalidData runtime error.
pub(super) use unix_core::invalid_data;

/// Return one ioPermissionDenied runtime error.
pub(super) use unix_core::permission_denied;

/// Resolve one runtime identifier for Android host callback routing.
pub(super) fn host_session_id(
    binding: &BindingCallContext,
    _operation: &'static str,
) -> Result<u64, Box<RuntimeError>> {
    Ok(binding.worker().runtime_id.0)
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
    let Some(status) = HostStatus::from_code(status) else {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} failed with status code {status}"),
        ));
    };

    match status {
        HostStatus::Ok => Ok(()),
        HostStatus::NotSupported => Err(core_platform::not_supported(operation)),
        HostStatus::InvalidArgument => Err(invalid_data(
            operation,
            format!("android host crypto {action} reported one invalid argument"),
        )),
        HostStatus::NotFound => Err(invalid_data(
            operation,
            format!("android host crypto {action} could not resolve one key"),
        )),
        HostStatus::PermissionDenied => Err(permission_denied(
            operation,
            format!("android host crypto {action} was denied"),
        )),
        HostStatus::BufferTooSmall => Err(invalid_data(
            operation,
            format!("android host crypto {action} reported one unexpectedly small output buffer"),
        )),
        HostStatus::Failed => Err(invalid_data(
            operation,
            format!("android host crypto {action} failed"),
        )),
        HostStatus::WouldBlock => Err(invalid_data(
            operation,
            format!("android host crypto {action} would block unexpectedly"),
        )),
    }
}
