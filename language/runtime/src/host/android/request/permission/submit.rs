use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};

/// Return one Android permission request outcome when supported.
pub(crate) fn submit_permission_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // FUGU #Architecture: mobile permission requests are still unwired in this Rust lane while the transaction path lands
        HostRequest::OsPermissionOpenSettings => Ok(None),
        // FUGU #Architecture: generic permission UI must move to one interactive host transaction path
        HostRequest::OsPermissionRequest { .. } => Ok(None),
        // FUGU #Architecture: generic permission UI must move to one interactive host transaction path
        HostRequest::OsPermissionRequestMany { .. } => Ok(None),
        _ => Ok(None),
    }
}
