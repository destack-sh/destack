use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::submit_test_contact_request;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::ActionSet;

/// Return dynamic Unix contact actions for Linux tests.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Linux contact request through the active test host.
pub(crate) fn submit_contact_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_contact_request(context, request)
}
