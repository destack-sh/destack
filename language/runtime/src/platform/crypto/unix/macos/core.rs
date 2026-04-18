use std::path::PathBuf;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation_sys::error::{CFErrorGetCode, CFErrorRef};
use core_foundation_sys::string::{CFStringCreateWithBytes, CFStringRef, kCFStringEncodingUTF8};
use security_framework_sys::base::{
    errSecAuthFailed, errSecBadReq, errSecIO, errSecInternalComponent, errSecItemNotFound,
    errSecParam, errSecUnimplemented,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use super::constants::{DEFAULT_MACOS_USER_KEYCHAIN_ACCOUNT, DEFAULT_MACOS_USER_KEYCHAIN_SERVICE};

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

/// Map one Security.framework CFError into one runtime error and release it.
pub(super) fn security_operation_error(
    operation: &'static str,
    action: &'static str,
    error: CFTypeRef,
) -> Box<RuntimeError> {
    // decode and release optional cf-error payload
    let status_code = if error.is_null() {
        None
    } else {
        let code = unsafe { CFErrorGetCode(error as CFErrorRef) as i32 };
        unsafe {
            CFRelease(error);
        }

        Some(code)
    };

    // map known security status codes into stable runtime lanes
    match status_code {
        Some(code)
            if code == errSecUnimplemented || code == errSecParam || code == errSecBadReq =>
        {
            core_platform::not_supported(operation)
        }
        Some(code)
            if code == errSecAuthFailed || code == errSecIO || code == errSecInternalComponent =>
        {
            permission_denied(
                operation,
                format!("{action} failed with security status code {code}"),
            )
        }
        Some(code) if code == errSecItemNotFound => core_platform::io_not_found(
            operation,
            format!("{action} failed because one keychain item was not found"),
        ),
        Some(code) => invalid_data(
            operation,
            format!("{action} failed with security status code {code}"),
        ),
        None => invalid_data(
            operation,
            format!("{action} failed with one unknown security error"),
        ),
    }
}

/// Return whether filesystem override mode is enabled for host store lanes.
pub(super) fn filesystem_mode_enabled(binding: &BindingCallContext) -> bool {
    configured_store_path(binding, CryptoStoreKind::User).is_some()
        || configured_store_path(binding, CryptoStoreKind::Machine).is_some()
}

/// Return whether one store lane is available in filesystem override mode.
pub(super) fn filesystem_store_lane_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    match kind {
        CryptoStoreKind::User => configured_store_path(binding, CryptoStoreKind::User).is_some(),
        CryptoStoreKind::Machine => {
            configured_store_path(binding, CryptoStoreKind::Machine).is_some()
        }
        CryptoStoreKind::Ephemeral => true,
        CryptoStoreKind::System | CryptoStoreKind::Provider => false,
    }
}

/// Return one configured host-store snapshot path for one lane.
pub(super) fn configured_store_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    match kind {
        CryptoStoreKind::User => binding.worker().options.crypto.host_store_paths.user.clone(),
        CryptoStoreKind::Machine => binding
            .worker()
            .options
            .crypto
            .host_store_paths
            .machine
            .clone(),
        _ => None,
    }
}

/// Return one configured macOS keychain service for host snapshot bytes.
pub(super) fn configured_keychain_snapshot_service(binding: &BindingCallContext) -> String {
    binding
        .worker()
        .options
        .crypto
        .macos_keychain_snapshot_service
        .clone()
        .unwrap_or_else(|| DEFAULT_MACOS_USER_KEYCHAIN_SERVICE.to_string())
}

/// Return one configured macOS keychain account for host snapshot bytes.
pub(super) fn configured_keychain_snapshot_account(binding: &BindingCallContext) -> String {
    binding
        .worker()
        .options
        .crypto
        .macos_keychain_snapshot_account
        .clone()
        .unwrap_or_else(|| DEFAULT_MACOS_USER_KEYCHAIN_ACCOUNT.to_string())
}

/// Decode one CFData payload into bytes.
pub(super) fn copy_cf_data_bytes(
    data: CFDataRef,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let pointer = unsafe { CFDataGetBytePtr(data) };
    let length = unsafe { CFDataGetLength(data) };
    if pointer.is_null() || length < 0 {
        return Err(invalid_data(
            operation,
            "failed to decode one CFData payload",
        ));
    }

    Ok(unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec())
}

/// Create one UTF-8 CoreFoundation string.
pub(super) fn create_cf_string(value: &str, operation: &'static str) -> RuntimeResult<CFStringRef> {
    let bytes = value.as_bytes();
    let string = unsafe {
        CFStringCreateWithBytes(
            kCFAllocatorDefault,
            bytes.as_ptr(),
            bytes.len() as isize,
            kCFStringEncodingUTF8,
            0,
        )
    };
    if string.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one CFString payload",
        ));
    }

    Ok(string)
}
