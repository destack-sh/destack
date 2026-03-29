use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::platform::abi::{NativeStringRef, NativeStringSlice};

/// Decode one optional callback string argument.
pub(super) fn decode_optional_string(
    is_present: bool,
    value: NativeStringRef,
    argument: &'static str,
) -> RuntimeResult<Option<String>> {
    if !is_present {
        return Ok(None);
    }

    decode_string(value, argument).map(Some)
}

/// Decode one callback string argument.
pub(super) fn decode_string(
    value: NativeStringRef,
    argument: &'static str,
) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(str::to_string)
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} string")))
}

/// Decode one callback string-slice argument.
pub(super) fn decode_string_slice(
    values: NativeStringSlice,
    argument: &'static str,
) -> RuntimeResult<Vec<String>> {
    let mut decoded_values = Vec::new();

    for value in unsafe { values.as_slice() }
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} slice")))?
    {
        decoded_values.push(decode_string(*value, argument)?);
    }

    Ok(decoded_values)
}

/// Convert one ingress callback result into a host status.
pub(super) fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
