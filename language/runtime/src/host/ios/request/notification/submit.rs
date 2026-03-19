use super::{
    destack_host_ios_notification_cancel, destack_host_ios_notification_cancel_all,
    destack_host_ios_notification_category_list, destack_host_ios_notification_category_set,
    destack_host_ios_notification_pending_cancel, destack_host_ios_notification_pending_cancel_all,
    destack_host_ios_notification_pending_list, destack_host_ios_notification_post,
    destack_host_ios_notification_request_permission, destack_host_ios_notification_schedule,
};
use crate::diagnostic::RuntimeResult;
use crate::host::callback::{
    decode_callback_host_status, encode_callback_host_json, read_buffered_callback_host_json,
    read_buffered_callback_host_string,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::PlatformError;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};
use crate::runtime::NativeSlice;

/// Return one iOS notification request outcome when supported.
pub(crate) fn submit_notification_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsNotificationRequestPermission => {
            let mut state = NotificationPermissionState::Prompt as i32;
            let status =
                unsafe { destack_host_ios_notification_request_permission(runtime_id, &mut state) };
            decode_callback_host_status(status, request.operation_name())?;
            let state = decode_notification_permission_state(state)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationPermissionState(state),
            )))
        }
        HostRequest::OsNotificationCancel { id } => {
            let id = id.as_bytes();
            let status = unsafe {
                destack_host_ios_notification_cancel(
                    runtime_id,
                    NativeSlice {
                        data: id.as_ptr() as *mut u8,
                        len: id.len() as u32,
                    },
                )
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationCancelAll => {
            let status = unsafe { destack_host_ios_notification_cancel_all(runtime_id) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationCategoryList => {
            let categories = read_buffered_callback_host_json::<Vec<NotificationCategoryValue>>(
                request.operation_name(),
                "notification categories",
                |output, output_written| unsafe {
                    destack_host_ios_notification_category_list(runtime_id, output, output_written)
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationCategories(categories),
            )))
        }
        HostRequest::OsNotificationCategorySet { categories } => {
            let payload = encode_callback_host_json(categories, request.operation_name())?;
            let status = unsafe {
                destack_host_ios_notification_category_set(
                    runtime_id,
                    NativeSlice {
                        data: payload.as_ptr() as *mut u8,
                        len: payload.len() as u32,
                    },
                )
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPendingList => {
            let descriptors = read_buffered_callback_host_json::<
                Vec<NotificationScheduledDescriptorValue>,
            >(
                request.operation_name(),
                "pending notifications",
                |output, output_written| unsafe {
                    destack_host_ios_notification_pending_list(runtime_id, output, output_written)
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationScheduledDescriptors(descriptors),
            )))
        }
        HostRequest::OsNotificationPendingCancel { id } => {
            let id = id.as_bytes();
            let status = unsafe {
                destack_host_ios_notification_pending_cancel(
                    runtime_id,
                    NativeSlice {
                        data: id.as_ptr() as *mut u8,
                        len: id.len() as u32,
                    },
                )
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPendingCancelAll => {
            let status = unsafe { destack_host_ios_notification_pending_cancel_all(runtime_id) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPost { request: value } => {
            let id = submit_notification_request_with_id(
                runtime_id,
                request.operation_name(),
                value,
                |runtime_id, payload, output_id, output_written| unsafe {
                    destack_host_ios_notification_post(
                        runtime_id,
                        payload,
                        output_id,
                        output_written,
                    )
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationId(id),
            )))
        }
        HostRequest::OsNotificationSchedule { request: value } => {
            let id = submit_notification_request_with_id(
                runtime_id,
                request.operation_name(),
                value,
                |runtime_id, payload, output_id, output_written| unsafe {
                    destack_host_ios_notification_schedule(
                        runtime_id,
                        payload,
                        output_id,
                        output_written,
                    )
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationId(id),
            )))
        }
        _ => Ok(None),
    }
}

/// Submit one buffered iOS notification request that returns one id string.
fn submit_notification_request_with_id(
    runtime_id: u64,
    operation: &'static str,
    request: &NotificationRequestValue,
    mut invoke: impl FnMut(u64, NativeSlice<u8>, NativeSlice<u8>, *mut u32) -> u32,
) -> RuntimeResult<String> {
    let payload = encode_callback_host_json(request, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_string(operation, |output_id, output_written| {
        invoke(runtime_id, payload, output_id, output_written)
    })
}

/// Decode one notification permission state integer from the callback host ABI.
fn decode_notification_permission_state(raw: i32) -> RuntimeResult<NotificationPermissionState> {
    match raw {
        1 => Ok(NotificationPermissionState::Granted),
        2 => Ok(NotificationPermissionState::Denied),
        3 => Ok(NotificationPermissionState::Prompt),
        _ => Err(PlatformError::invalid_argument_value(
            "state",
            "unknown notification permission state",
        )
        .into()),
    }
}
