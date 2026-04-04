use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, HostSessionId, RequestContext};
use crate::platform::os::{Permission, PermissionState};

/// Fall through on non-Windows test builds that only compile the Windows host tree.
pub(crate) fn submit_location_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}

/// Fall through on non-Windows test builds that only compile the Windows host tree.
pub(crate) fn request_location_permission(
    _context: &RequestContext,
    _permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    Ok(None)
}

/// Ignore Windows location cleanup on non-Windows test builds.
pub(crate) fn unregister_location_runtime(_host_runtime_id: HostSessionId) {}
