use crate::diagnostic::RuntimeResult;
use crate::host::abi::document::HostDocumentRequestPayload;
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::document::ffi::destack_host_ios_document_pick;

/// Return one iOS document request outcome when supported.
pub(crate) fn submit_document_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsDocumentPick { options } => {
            let payload = HostDocumentRequestPayload::new(context.request_id.0, options);
            let status =
                unsafe { destack_host_ios_document_pick(context.host_session_id.0, payload.abi()) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::event_completing(
                HostRequestResult::DocumentDescriptors(Vec::new()),
            )))
        }
        _ => Ok(None),
    }
}
