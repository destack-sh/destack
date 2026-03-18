use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};

/// Fall through on non-Windows test builds that only compile the Windows host tree.
pub(crate) fn submit_calendar_request(
    _context: &HostRequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
