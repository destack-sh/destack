use crate::diagnostic::RuntimeResult;
use crate::host::abi::document::HostDocumentResult;
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::core::HostRequestId;
use crate::host::{HostDocumentEvent, HostEvent};
use crate::platform::NativeAbiCodec;

/// Submit one iOS document-result callback.
pub(crate) fn ios_notify_document_result(
    session_handle: u64,
    result: HostDocumentResult,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let documents = unsafe { result.documents.as_slice() }?
        .iter()
        .copied()
        .map(|document| unsafe { document.into_value() })
        .collect::<RuntimeResult<Vec<_>>>()?;

    queue.enqueue(HostEvent::Document(Box::new(HostDocumentEvent {
        request_id: HostRequestId(result.request_id),
        documents,
    })));

    Ok(())
}
