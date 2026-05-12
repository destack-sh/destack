use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request as unix_request;
use crate::host::{HostRequest, HostRequestOutcome, HostSessionId, RequestContext};
use crate::runtime::action::ActionSet;

/// Return dynamic Linux request actions.
pub(crate) fn request_actions() -> ActionSet {
    unix_request::request_actions()
}

/// Submit one normalized Linux host request.
pub(crate) fn submit_request(
    context: &RequestContext,
    request: HostRequest,
) -> RuntimeResult<HostRequestOutcome> {
    unix_request::submit_request(context, request)
}

/// Remove one Linux location runtime from the active backend.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    unix_request::location::unregister_location_runtime(host_session_id);
}
