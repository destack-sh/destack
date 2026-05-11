use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::HostActionSet;

/// Return no Unix contact actions on unsupported hosts.
pub(crate) fn request_actions() -> HostActionSet {
    HostActionSet::default()
}

/// Decline Unix contact requests on unsupported hosts.
pub(crate) fn submit_contact_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
