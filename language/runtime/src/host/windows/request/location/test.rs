use crate::diagnostic::RuntimeResult;
use crate::host::os::windows::tests::{
    request_location_permission as request_location_permission_from_tests,
    submit_location_request as submit_location_request_from_tests,
    unregister_location_runtime as unregister_location_runtime_from_tests,
};
use crate::host::{HostRequest, HostRequestOutcome, HostSessionId, RequestContext};
use crate::platform::os::{Permission, PermissionState};

/// Submit one Windows location request through the active test lane.
pub(crate) fn submit_location_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_location_request_from_tests(context, request)
}

/// Request one Windows location permission through the active test lane.
pub(crate) fn request_location_permission(
    context: &RequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    request_location_permission_from_tests(context, permission)
}

/// Remove one Windows runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    unregister_location_runtime_from_tests(host_session_id);
}
