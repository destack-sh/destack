use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::runtime::action::ActionSet;

/// Return no Unix contact actions on unsupported hosts.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::default()
}

/// Decline Unix contact requests on unsupported hosts.
pub(crate) fn submit_contact_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
