use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::tests::{
    submit_test_location_request, unregister_test_location_runtime,
};
use crate::host::{HostRequest, HostRequestOutcome, HostSessionId, RequestContext};
use crate::runtime::action::ActionSet;

/// Return dynamic Unix location actions for Linux tests.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Linux location request through the active test host.
pub(crate) fn submit_location_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_location_request(context, request)
}

/// Remove one Linux runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    unregister_test_location_runtime(host_session_id);
}
