use crate::diagnostic::RuntimeResult;
pub(crate) use crate::host::os::macos::tests::{MacosLocationHooks, set_macos_location_test_hooks};
use crate::host::os::macos::tests::{
    request_location_permission as request_location_permission_from_tests,
    submit_location_request as submit_location_request_from_tests,
    unregister_location_runtime as unregister_location_runtime_from_tests,
};
use crate::host::{HostRequest, HostRequestOutcome, HostSessionId, RequestContext};
use crate::platform::os::{Permission, PermissionState};

/// Submit one macOS location request through the active test lane.
pub(crate) fn submit_location_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_location_request_from_tests(context, request)
}

/// Request one macOS location permission through the active test lane.
pub(crate) fn request_location_permission(
    context: &RequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    request_location_permission_from_tests(context, permission)
}

/// Remove one macOS runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    unregister_location_runtime_from_tests(host_session_id);
}
