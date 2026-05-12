use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::submit_test_calendar_request;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::ActionSet;

/// Return dynamic Unix calendar request actions for Linux tests.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Unix calendar request through the Linux test host.
pub(crate) fn submit_calendar_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_calendar_request(context, request)
}
