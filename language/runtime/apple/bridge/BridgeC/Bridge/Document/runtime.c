#include "Bridge/Types.h"
#include "Bridge/Document/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one document result into the runtime ingress path.
DestackRustRuntimeStatus send_document_pick_result(
    uint64_t session_handle,
    uint64_t request_id,
    DestackRustDocumentDescriptorSlice documents
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_document_result(session_handle, request_id, documents);
}
