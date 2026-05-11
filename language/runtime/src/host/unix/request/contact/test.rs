use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::submit_test_contact_request;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::{HostAction, HostActionSet};

/// Return dynamic Unix contact actions for Linux tests.
pub(crate) fn request_actions() -> HostActionSet {
    HostActionSet::from_actions([HostAction::OsContactRead, HostAction::OsContactWrite])
}

/// Submit one Linux contact request through the active test lane.
pub(crate) fn submit_contact_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    submit_test_contact_request(context, request)
}
