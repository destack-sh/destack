use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, RequestContext};
use crate::platform::core::not_supported;
use crate::runtime::action::HostActionSet;

/// Return the empty action set for unsupported Unix location backends.
pub(crate) fn request_actions() -> HostActionSet {
    HostActionSet::default()
}

/// Reject Unix location requests on hosts without a concrete provider.
pub(crate) fn submit_location_request(
    _context: &RequestContext,
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
