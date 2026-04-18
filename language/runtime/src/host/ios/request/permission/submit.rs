use crate::diagnostic::RuntimeResult;
use crate::host::abi::permission::{HostPermission, HostPermissionRequest};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::os::apple::abi::permission::ffi::{
    destack_host_ios_permission_open_settings, destack_host_ios_permission_request,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::NativeAbiCodec;
use crate::platform::os::PermissionState;
use crate::runtime::BindingCallContext;

/// Return one iOS permission request outcome when supported.
pub(crate) fn submit_permission_request(
    context: &RequestContext,
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
            let binding = BindingCallContext::from_current_worker_for_native()?;
            let abi_request = HostPermissionRequest {
                request_id: context.request_id.0,
                permission: <HostPermission as NativeAbiCodec>::from_value(&binding, *permission),
            };
            let status = unsafe {
                destack_host_ios_permission_request(context.host_session_id.0, abi_request)
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::event_completing(
                HostRequestResult::PermissionState(PermissionState::Prompt),
            )))
        }
        _ => Ok(None),
    }
}
