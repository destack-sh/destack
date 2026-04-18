use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::background::{
    HostBackgroundCompleteRequest, HostBackgroundListResponse, HostBackgroundStatus,
    HostBackgroundStatusResponse, HostBackgroundTaskOptions, HostBackgroundTaskResult,
    HostBackgroundTriggerTestRequest, HostBackgroundTriggerTestResponse,
    HostBackgroundUnregisterRequest,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::os::android::abi::background::{
    destack_host_android_background_complete, destack_host_android_background_list,
    destack_host_android_background_register_task, destack_host_android_background_status,
    destack_host_android_background_trigger_test, destack_host_android_background_unregister,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::NativeAbiCodec;
use crate::platform::abi::NativeStringRef;
use crate::runtime::BindingCallContext;

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
            let status = unsafe { response.scheduler_status.into_value()? };
            let status = match status {
                Some(status) => status,
                None => unsafe {
                    <HostBackgroundStatus as NativeAbiCodec>::into_value(
                        HostBackgroundStatus::Unavailable,
                    )?
                },
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
            let descriptors_value = unsafe { response.descriptors.into_value()? };

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundTaskDescriptors(descriptors_value),
            )))
        }
        HostRequest::OsBackgroundRegister { options } => {
            let binding = BindingCallContext::from_current_worker_for_native()?;
            let options = HostBackgroundTaskOptions::from_value(&binding, options.clone());
            let call_status =
                unsafe { destack_host_android_background_register_task(runtime_id, options) };
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
            let binding = BindingCallContext::from_current_worker_for_native()?;
            let payload = HostBackgroundCompleteRequest {
                execution_id: NativeStringRef::from(execution_id),
                result: <HostBackgroundTaskResult as NativeAbiCodec>::from_value(&binding, *result),
            };
            let call_status =
                unsafe { destack_host_android_background_complete(runtime_id, payload) };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
