use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation_sys::error::{CFErrorGetCode, CFErrorRef};
use core_foundation_sys::string::{CFStringCreateWithBytes, CFStringRef, kCFStringEncodingUTF8};
use security_framework_sys::base::{
    errSecAuthFailed, errSecBadReq, errSecIO, errSecInternalComponent, errSecItemNotFound,
    errSecParam, errSecUnimplemented,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

/// Return one ioInvalidData runtime error.
pub(crate) fn invalid_data(
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
pub(crate) fn permission_denied(
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
pub(crate) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Return one ioNotFound runtime error.
pub(crate) fn not_found(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Map one Security.framework CFError into one runtime error and release it.
pub(crate) fn security_operation_error(
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
            not_supported(operation)
        }
        Some(code)
            if code == errSecAuthFailed || code == errSecIO || code == errSecInternalComponent =>
        {
            permission_denied(
                operation,
                format!("{action} failed with security status code {code}"),
            )
        }
        Some(code) if code == errSecItemNotFound => not_found(
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

/// Decode one CFData payload into bytes.
pub(crate) fn copy_cf_data_bytes(
    data: CFDataRef,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // decode pointer and length from CFData
    let pointer = unsafe { CFDataGetBytePtr(data) };
    let length = unsafe { CFDataGetLength(data) };
    if pointer.is_null() || length < 0 {
        return Err(invalid_data(
            operation,
            "failed to decode one CFData payload",
        ));
    }

    // copy byte payload into owned runtime memory
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec();

    Ok(bytes)
}

/// Create one CFString from one UTF-8 Rust string.
pub(crate) fn create_cf_string(value: &str, operation: &'static str) -> RuntimeResult<CFStringRef> {
    // encode utf-8 bytes into CFString
    let value_bytes = value.as_bytes();
    let string = unsafe {
        CFStringCreateWithBytes(
            kCFAllocatorDefault,
            value_bytes.as_ptr(),
            value_bytes.len() as isize,
            kCFStringEncodingUTF8,
            false as u8,
        )
    };
    if string.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one keychain string value",
        ));
    }

    Ok(string)
}
