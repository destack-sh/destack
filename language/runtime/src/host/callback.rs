#![cfg_attr(test, allow(dead_code))]

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::not_supported;
use crate::host::core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{NativeSlice, PlatformError};

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

/// Serialize one callback-host payload into JSON bytes.
pub(crate) fn encode_callback_host_json<T: Serialize>(
    value: &T,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} could not encode one host payload: {error}"),
        ))
        .boxed()
    })
}

/// Deserialize one callback-host payload from JSON bytes.
pub(crate) fn decode_callback_host_json<T: DeserializeOwned>(
    bytes: &[u8],
    operation: &'static str,
    payload: &'static str,
) -> RuntimeResult<T> {
    serde_json::from_slice(bytes).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} returned one invalid {payload} payload: {error}"),
        ))
        .boxed()
    })
}

/// Read one buffered callback-host payload into one owned byte vector.
pub(crate) fn read_buffered_callback_host_output(
    operation: &'static str,
    mut invoke: impl FnMut(NativeSlice<u8>, *mut u32) -> u32,
) -> RuntimeResult<Vec<u8>> {
    let mut output_written = 0_u32;
    let first_status = invoke(
        NativeSlice {
            data: std::ptr::null_mut(),
            len: 0,
        },
        &mut output_written,
    );

    if first_status != HOST_STATUS_BUFFER_TOO_SMALL {
        decode_callback_host_status(first_status, operation)?;

        return Ok(Vec::new());
    }

    let mut output = vec![0_u8; output_written as usize];
    let second_status = invoke(
        NativeSlice {
            data: output.as_mut_ptr(),
            len: output.len() as u32,
        },
        &mut output_written,
    );
    decode_callback_host_status(second_status, operation)?;
    output.truncate(output_written as usize);

    Ok(output)
}

/// Read one string payload from one buffered callback-host call.
pub(crate) fn read_buffered_callback_host_string(
    operation: &'static str,
    invoke: impl FnMut(NativeSlice<u8>, *mut u32) -> u32,
) -> RuntimeResult<String> {
    let output = read_buffered_callback_host_output(operation, invoke)?;

    String::from_utf8(output).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} returned one invalid utf8 payload: {error}"),
        ))
        .boxed()
    })
}

/// Read one JSON payload from one buffered callback-host call.
pub(crate) fn read_buffered_callback_host_json<T: DeserializeOwned>(
    operation: &'static str,
    payload: &'static str,
    invoke: impl FnMut(NativeSlice<u8>, *mut u32) -> u32,
) -> RuntimeResult<T> {
    let output = read_buffered_callback_host_output(operation, invoke)?;

    decode_callback_host_json(&output, operation, payload)
}
