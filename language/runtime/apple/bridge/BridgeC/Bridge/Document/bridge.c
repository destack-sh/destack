#include "Bridge/Types.h"
#include "Bridge/Document/Runtime.h"

/// Deliver one document result into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_document_result(
    uint64_t session_handle,
    uint64_t request_id,
    DestackRustDocumentDescriptorSlice documents
) {
    return send_document_pick_result(session_handle, request_id, documents);
}
