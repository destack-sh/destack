use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::platform::core::not_supported;
use crate::runtime::capability::PlatformCapabilitySet;

/// Return the empty capability set for unsupported Unix location backends.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    PlatformCapabilitySet::default()
}

/// Reject Unix location requests on hosts without a concrete provider.
pub(crate) fn submit_location_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsLocationServicesEnabled
        | HostRequest::OsLocationLastKnown
        | HostRequest::OsLocationWatchOpen { .. }
        | HostRequest::OsLocationWatchClose { .. } => Err(not_supported(request.operation_name())),
        _ => Ok(None),
    }
}
