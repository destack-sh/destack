use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::not_supported;
use crate::host::core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
use crate::platform::PlatformError;
use crate::platform::abi::NativeStringRef;
use crate::platform::diagnostic::PlatformErrorCode;

/// Decode one callback-host status code into one runtime result.
pub(crate) fn decode_callback_host_status(
    status: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    match status {
        HOST_STATUS_OK => Ok(()),
        HOST_STATUS_NOT_SUPPORTED => Err(not_supported(operation)),
        HOST_STATUS_INVALID_ARGUMENT => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "request",
                format!("{operation} rejected one invalid argument"),
            ))
            .boxed())
        }
        HOST_STATUS_NOT_FOUND => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} could not resolve one runtime-scoped callback host binding"),
        ))
        .boxed()),
        HOST_STATUS_PERMISSION_DENIED => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("{operation} was denied by the callback host"),
        ))
        .boxed()),
        HOST_STATUS_BUFFER_TOO_SMALL => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} reported one unexpected buffer negotiation"),
        ))
        .boxed()),
        HOST_STATUS_FAILED => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} failed in the callback host"),
        ))
        .boxed()),
        _ => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} returned one unknown callback host status"),
        ))
        .boxed()),
    }
}

/// Decode one required string reference from one callback-host call.
pub(crate) fn decode_callback_host_required_string_ref(
    operation: &'static str,
    payload: &'static str,
    value: NativeStringRef,
) -> RuntimeResult<String> {
    let value = unsafe { value.as_str() }?;

    if value.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            payload,
            format!("{operation} returned one empty {payload}"),
        ))
        .boxed());
    }

    Ok(value.to_owned())
}
