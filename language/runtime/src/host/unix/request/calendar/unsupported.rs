use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::ActionSet;

/// Return no Unix calendar actions on unsupported hosts.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::default()
}

/// Reject Unix calendar requests on unsupported hosts.
pub(crate) fn submit_calendar_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
