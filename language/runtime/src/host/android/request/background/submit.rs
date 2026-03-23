use crate::diagnostic::RuntimeResult;
use crate::host::android::abi::background::{
    destack_host_android_background_complete, destack_host_android_background_list,
    destack_host_android_background_register, destack_host_android_background_status,
    destack_host_android_background_trigger_test, destack_host_android_background_unregister,
};
use crate::host::core::callback::{
    decode_callback_host_status, encode_callback_host_json, read_buffered_callback_host_json,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::PlatformError;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
    BackgroundTaskResultValue,
};
use crate::runtime::NativeSlice;

/// Return one Android background request outcome when supported.
pub(crate) fn submit_background_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsBackgroundStatus => {
            let mut status = BackgroundStatusValue::Unavailable as i32;
            let call_status =
                unsafe { destack_host_android_background_status(runtime_id, &mut status) };
            decode_callback_host_status(call_status, request.operation_name())?;
            let status = decode_background_status(status)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundStatus(status),
            )))
        }
        HostRequest::OsBackgroundList => {
            let descriptors = read_buffered_callback_host_json::<Vec<BackgroundTaskDescriptorValue>>(
                request.operation_name(),
                "background task descriptors",
                |output, output_written| unsafe {
                    destack_host_android_background_list(runtime_id, output, output_written)
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundTaskDescriptors(descriptors),
            )))
        }
        HostRequest::OsBackgroundRegister { options } => {
            let payload = encode_callback_host_json(options, request.operation_name())?;
            let call_status = unsafe {
                destack_host_android_background_register(
                    runtime_id,
                    NativeSlice {
                        data: payload.as_ptr() as *mut u8,
                        len: payload.len() as u32,
                    },
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundUnregister { identifier } => {
            let identifier = identifier.as_bytes();
            let call_status = unsafe {
                destack_host_android_background_unregister(
                    runtime_id,
                    NativeSlice {
                        data: identifier.as_ptr() as *mut u8,
                        len: identifier.len() as u32,
                    },
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundTriggerTest { identifier } => {
            let identifier = identifier.as_bytes();
            let mut is_triggered = false;
            let call_status = unsafe {
                destack_host_android_background_trigger_test(
                    runtime_id,
                    NativeSlice {
                        data: identifier.as_ptr() as *mut u8,
                        len: identifier.len() as u32,
                    },
                    &mut is_triggered,
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_triggered),
            )))
        }
        HostRequest::OsBackgroundComplete {
            execution_id,
            result,
        } => {
            let execution_id = execution_id.as_bytes();
            let call_status = unsafe {
                destack_host_android_background_complete(
                    runtime_id,
                    NativeSlice {
                        data: execution_id.as_ptr() as *mut u8,
                        len: execution_id.len() as u32,
                    },
                    encode_background_result(*result),
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Decode one background status integer from the callback host ABI.
fn decode_background_status(raw: i32) -> RuntimeResult<BackgroundStatusValue> {
    match raw {
        1 => Ok(BackgroundStatusValue::Unavailable),
        2 => Ok(BackgroundStatusValue::Restricted),
        3 => Ok(BackgroundStatusValue::Available),
        _ => {
            Err(PlatformError::invalid_argument_value("status", "unknown background status").into())
        }
    }
}

/// Encode one background result enum for the callback host ABI.
fn encode_background_result(result: BackgroundTaskResultValue) -> i32 {
    match result {
        BackgroundTaskResultValue::Success => 1,
        BackgroundTaskResultValue::Retry => 2,
        BackgroundTaskResultValue::Failure => 3,
    }
}
