use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::HostActionSet;

/// Return no Unix calendar actions on unsupported hosts.
pub(crate) fn request_actions() -> HostActionSet {
    HostActionSet::default()
}

/// Reject Unix calendar requests on unsupported hosts.
pub(crate) fn submit_calendar_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
