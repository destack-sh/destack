use super::callbacks::call_android_document_callback;
use crate::host::abi::document::HostDocumentRequest;

/// Open one Android document picker request.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_document_pick(
    runtime_id: u64,
    request: HostDocumentRequest,
) -> u32 {
    call_android_document_callback(
        runtime_id,
        |callbacks| callbacks.pick,
        |callback| unsafe { callback(runtime_id, request) },
    )
}
