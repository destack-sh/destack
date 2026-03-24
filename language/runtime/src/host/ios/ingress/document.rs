use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestId, HostSessionHandle};
use crate::host::ios::ingress::core::ios_host_queue;
use crate::host::{HostDocumentEvent, HostEvent};
use crate::platform::os::abi_generated::DocumentDescriptorValue;

/// Submit one iOS document-result callback.
pub(crate) fn ios_notify_document_result(
    session_handle: HostSessionHandle,
    request_id: u64,
    documents: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Document(Box::new(HostDocumentEvent {
        request_id: HostRequestId(request_id),
        documents,
    })));

    Ok(())
}
