use crate::diagnostic::RuntimeResult;
use crate::host::abi::document::HostDocumentResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostRequestId;
use crate::host::{HostDocumentEvent, HostEvent};
use crate::platform::NativeAbiCodec;

/// Submit one Android document-result callback.
pub(crate) fn android_notify_document_result(
    session_handle: u64,
    result: HostDocumentResult,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;
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
