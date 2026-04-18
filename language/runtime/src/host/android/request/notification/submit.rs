use crate::diagnostic::RuntimeResult;
use crate::host::abi::notification::HostNotificationRequest;
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::error::not_supported;
use crate::host::os::android::abi::notification::{
    destack_host_android_notification_cancel, destack_host_android_notification_cancel_all,
    destack_host_android_notification_post,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::NativeAbiCodec;
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::NotificationRequestValue;
use crate::runtime::BindingCallContext;

/// Return one Android notification request outcome when supported.
pub(crate) fn submit_notification_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsNotificationCancel { id } => {
            let identifier = NativeStringRef::from(id.as_str());
            let status =
                unsafe { destack_host_android_notification_cancel(runtime_id, identifier) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationCancelAll => {
            let status = unsafe { destack_host_android_notification_cancel_all(runtime_id) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsNotificationPost { request: value } => {
            let id = submit_notification_post(runtime_id, request.operation_name(), value)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::NotificationId(id),
            )))
        }
        HostRequest::OsNotificationRequestPermission
        | HostRequest::OsNotificationCategoryList
        | HostRequest::OsNotificationCategorySet { .. }
        | HostRequest::OsNotificationPendingList
        | HostRequest::OsNotificationPendingCancel { .. }
        | HostRequest::OsNotificationPendingCancelAll
        | HostRequest::OsNotificationSchedule { .. } => {
            Err(not_supported(request.operation_name()).into())
        }
        _ => Ok(None),
    }
}

/// Submit one Android mobile notification request.
fn submit_notification_post(
    runtime_id: u64,
    operation: &'static str,
    request: &NotificationRequestValue,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_worker_for_native()?;
    let request = HostNotificationRequest::from_value(&binding, request.clone());
    let status = unsafe { destack_host_android_notification_post(runtime_id, request) };
    decode_callback_host_status(status, operation)?;

    Ok(unsafe { request.tag.as_str()? }.to_string())
}
