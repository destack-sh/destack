use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};

/// Fall through on non-macOS test builds that only compile the macOS host tree.
pub(crate) fn submit_contact_request(
    _context: &RequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
