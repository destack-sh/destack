use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::{HostRequestId, HostSessionHandle};
use crate::host::{HostDocumentEvent, HostEvent};
use crate::platform::os::abi_generated::DocumentDescriptorValue;

/// Submit one Android document-result callback.
pub(crate) fn android_notify_document_result(
    session_handle: HostSessionHandle,
    request_id: u64,
    documents: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Document(Box::new(HostDocumentEvent {
        request_id: HostRequestId(request_id),
        documents,
    })));

    Ok(())
}
