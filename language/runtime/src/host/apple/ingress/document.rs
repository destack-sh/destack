use crate::diagnostic::RuntimeResult;
use crate::host::abi::document::HostDocumentResult;
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::{
    HostEvent, HostRequestCompletionEvent, HostRequestId, HostRequestResult, HostSessionHandle,
};
use crate::platform::NativeAbiCodec;

/// Submit one iOS document-result callback.
pub(crate) fn ios_notify_document_result(
    session_handle: HostSessionHandle,
    result: HostDocumentResult,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let request_id = result.request_id;
    let documents = unsafe { result.documents.into_value()? };

    queue.enqueue(HostEvent::RequestCompletion(HostRequestCompletionEvent {
        request_id: HostRequestId(request_id),
        result: HostRequestResult::DocumentDescriptors(documents),
    }));

    Ok(())
}
