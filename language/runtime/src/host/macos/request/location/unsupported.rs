use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
use crate::platform::os::{Permission, PermissionState};

/// Fall through on non-macOS builds that only compile the macOS host tree.
pub(crate) fn request_location_permission(
    _context: &HostRequestContext,
    _permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    Ok(None)
}

/// Fall through on non-macOS test builds that only compile the macOS host tree.
pub(crate) fn submit_location_request(
    _context: &HostRequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}

/// Remove one macOS runtime from non-macOS test builds.
pub(crate) fn unregister_location_runtime(_host_runtime_id: HostRuntimeId) {}
