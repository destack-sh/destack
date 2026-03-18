use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::runtime::capability::PlatformCapabilitySet;

/// Return no Unix contact capabilities on unsupported hosts.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::default()
}

/// Decline Unix contact requests on unsupported hosts.
pub(crate) fn submit_contact_request(
    _context: &HostRequestContext,
    _request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    Ok(None)
}
