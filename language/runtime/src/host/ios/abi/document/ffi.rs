use super::callbacks::call_ios_document_callback;
use crate::host::abi::document::HostDocumentRequest;

/// Open one iOS document picker request.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_document_pick(
    runtime_id: u64,
    request: HostDocumentRequest,
) -> u32 {
    call_ios_document_callback(
        runtime_id,
        |callbacks| callbacks.pick,
        |callback| unsafe { callback(runtime_id, request) },
    )
}
