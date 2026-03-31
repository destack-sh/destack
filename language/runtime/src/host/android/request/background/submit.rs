use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::background::{
    HostBackgroundCompleteRequest, HostBackgroundListResponse, HostBackgroundStatusResponse,
    HostBackgroundTaskOptionsPayload, HostBackgroundTriggerTestRequest,
    HostBackgroundTriggerTestResponse, HostBackgroundUnregisterRequest, decode_descriptor,
    decode_status, encode_result,
};
use crate::host::android::abi::background::{
    destack_host_android_background_complete, destack_host_android_background_list,
    destack_host_android_background_register_task, destack_host_android_background_status,
    destack_host_android_background_trigger_test, destack_host_android_background_unregister,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::abi::NativeStringRef;

/// Return one Android background request outcome when supported.
pub(crate) fn submit_background_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsBackgroundStatus => {
            // callback response
            let mut response = MaybeUninit::<HostBackgroundStatusResponse>::uninit();
            let call_status = unsafe {
                destack_host_android_background_status(runtime_id, response.as_mut_ptr())
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            let response = unsafe { response.assume_init() };
            decode_callback_host_status(response.status, request.operation_name())?;

            // decode returned payload
            let status = if response.has_scheduler_status {
                decode_status(response.scheduler_status)
            } else {
                decode_status(crate::host::abi::background::HostBackgroundStatus::Unavailable)
            };

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundStatus(status),
            )))
        }
        HostRequest::OsBackgroundList => {
            // callback response
            let mut response = MaybeUninit::<HostBackgroundListResponse>::uninit();
            let call_status =
                unsafe { destack_host_android_background_list(runtime_id, response.as_mut_ptr()) };
            decode_callback_host_status(call_status, request.operation_name())?;

            let response = unsafe { response.assume_init() };
            decode_callback_host_status(response.status, request.operation_name())?;

            // decode returned payload
            let descriptors = unsafe { response.descriptors.as_slice() }?;
            let mut descriptors_value = Vec::with_capacity(descriptors.len());

            // decode each background task descriptor
            for descriptor in descriptors {
                descriptors_value.push(decode_descriptor(*descriptor)?);
            }

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundTaskDescriptors(descriptors_value),
            )))
        }
        HostRequest::OsBackgroundRegister { options } => {
            let options = HostBackgroundTaskOptionsPayload::new(options);
            let call_status =
                unsafe { destack_host_android_background_register_task(runtime_id, options.abi()) };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundUnregister { identifier } => {
            let payload = HostBackgroundUnregisterRequest {
                identifier: NativeStringRef::from(identifier),
            };
            let call_status =
                unsafe { destack_host_android_background_unregister(runtime_id, payload) };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundTriggerTest { identifier } => {
            // callback request and response
            let payload = HostBackgroundTriggerTestRequest {
                identifier: NativeStringRef::from(identifier),
            };
            let mut response = MaybeUninit::<HostBackgroundTriggerTestResponse>::uninit();
            let call_status = unsafe {
                destack_host_android_background_trigger_test(
                    runtime_id,
                    payload,
                    response.as_mut_ptr(),
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            let response = unsafe { response.assume_init() };
            decode_callback_host_status(response.status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(response.is_triggered),
            )))
        }
        HostRequest::OsBackgroundComplete {
            execution_id,
            result,
        } => {
            let payload = HostBackgroundCompleteRequest {
                execution_id: NativeStringRef::from(execution_id),
                result: encode_result(*result),
            };
            let call_status =
                unsafe { destack_host_android_background_complete(runtime_id, payload) };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
