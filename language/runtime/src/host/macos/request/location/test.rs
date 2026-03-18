use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
pub(crate) use crate::host::macos::tests::{MacosLocationHooks, set_macos_location_test_hooks};
use crate::host::macos::tests::{
    request_location_permission as request_location_permission_from_tests,
    submit_location_request as submit_location_request_from_tests,
    unregister_location_runtime as unregister_location_runtime_from_tests,
};
use crate::platform::os::{Permission, PermissionState};

/// Submit one macOS location request through the active test lane.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_location_request_from_tests(context, request)
}

/// Request one macOS location permission through the active test lane.
pub(crate) fn request_location_permission(
    context: &HostRequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    request_location_permission_from_tests(context, permission)
}

/// Remove one macOS runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_location_runtime_from_tests(host_runtime_id);
}
