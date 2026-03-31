use crate::diagnostic::RuntimeResult;
use crate::host::abi::permission::HostPermissionRequestPayload;
use crate::host::apple::abi::permission::ffi::{
    destack_host_ios_permission_open_settings, destack_host_ios_permission_request,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::platform::os::PermissionState;

/// Return one iOS permission request outcome when supported.
pub(crate) fn submit_permission_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsPermissionOpenSettings => {
            let status =
                unsafe { destack_host_ios_permission_open_settings(context.host_session_id.0) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsPermissionRequest { permission } => {
            let payload = HostPermissionRequestPayload::single(context.request_id.0, *permission);
            let status = unsafe {
                destack_host_ios_permission_request(context.host_session_id.0, payload.abi())
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::event_completing(
                HostRequestResult::PermissionState(PermissionState::Prompt),
            )))
        }
        _ => Ok(None),
    }
}
