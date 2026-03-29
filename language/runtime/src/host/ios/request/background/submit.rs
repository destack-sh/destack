use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::background::{
    HostBackgroundStatus, HostBackgroundTaskDescriptor, HostBackgroundTaskOptionsPayload,
    decode_descriptors, decode_status, encode_result,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::background::{
    destack_host_ios_background_complete, destack_host_ios_background_list,
    destack_host_ios_background_register_task, destack_host_ios_background_status,
    destack_host_ios_background_trigger_test, destack_host_ios_background_unregister,
};
use crate::platform::NativeArray;
use crate::platform::abi::NativeStringRef;

/// Return one iOS background request outcome when supported.
pub(crate) fn submit_background_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsBackgroundStatus => {
            let mut status = MaybeUninit::<HostBackgroundStatus>::uninit();
            let call_status =
                unsafe { destack_host_ios_background_status(runtime_id, status.as_mut_ptr()) };
            decode_callback_host_status(call_status, request.operation_name())?;
            let status = unsafe { status.assume_init() };
            let status = decode_status(status);

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundStatus(status),
            )))
        }
        HostRequest::OsBackgroundList => {
            let mut descriptors =
                MaybeUninit::<NativeArray<HostBackgroundTaskDescriptor>>::uninit();
            let call_status =
                unsafe { destack_host_ios_background_list(runtime_id, descriptors.as_mut_ptr()) };
            decode_callback_host_status(call_status, request.operation_name())?;

            let descriptors = unsafe { descriptors.assume_init() };
            let descriptors = unsafe { decode_descriptors(descriptors) }?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::BackgroundTaskDescriptors(descriptors),
            )))
        }
        HostRequest::OsBackgroundRegister { options } => {
            let options = HostBackgroundTaskOptionsPayload::new(options);
            let call_status =
                unsafe { destack_host_ios_background_register_task(runtime_id, options.abi()) };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundUnregister { identifier } => {
            let call_status = unsafe {
                destack_host_ios_background_unregister(
                    runtime_id,
                    NativeStringRef::from(identifier),
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundTriggerTest { identifier } => {
            let mut is_triggered = false;
            let call_status = unsafe {
                destack_host_ios_background_trigger_test(
                    runtime_id,
                    NativeStringRef::from(identifier),
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
            let call_status = unsafe {
                destack_host_ios_background_complete(
                    runtime_id,
                    NativeStringRef::from(execution_id),
                    encode_result(*result),
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
