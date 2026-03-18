use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::runtime::capability::PlatformCapabilitySet;

/// Return no Unix calendar capabilities on unsupported hosts.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::default()
}

/// Reject Unix calendar requests on unsupported hosts.
pub(crate) fn submit_calendar_request(
    _context: &HostRequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
