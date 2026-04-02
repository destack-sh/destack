use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::platform::abi::NativeStringRef;

/// Decode one callback string argument.
pub(super) fn decode_string(
    value: NativeStringRef,
    argument: &'static str,
) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(str::to_string)
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} string")))
}

/// Convert one ingress callback result into a host status.
pub(super) fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
