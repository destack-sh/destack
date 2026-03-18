use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};

/// Fall through on non-macOS test builds that only compile the macOS host tree.
pub(crate) fn submit_contact_request(
    _context: &HostRequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
