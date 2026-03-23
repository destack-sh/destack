use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostSessionId};
use crate::host::unix::request as unix_request;
use crate::runtime::capability::PlatformCapabilitySet;

/// Return dynamic Linux request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    unix_request::request_capabilities()
}

/// Submit one normalized Linux host request.
pub(crate) fn submit_request(
    context: &HostRequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    unix_request::submit_request(context, request)
}

/// Remove one Linux location runtime from the active backend.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    unix_request::location::unregister_location_runtime(host_session_id);
}
