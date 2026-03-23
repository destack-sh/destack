use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};

/// Return one Android document request outcome when supported.
pub(crate) fn submit_document_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // FUGU #Architecture: document pick is one interactive host transaction and must move to one async transaction path
        HostRequest::OsDocumentPick { .. } => Ok(None),
        _ => Ok(None),
    }
}
