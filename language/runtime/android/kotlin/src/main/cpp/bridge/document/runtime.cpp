#include "../types.h"
#include "../loader.h"
#include "runtime.h"

namespace {

/// Convert one host status code into one runtime status.
RuntimeStatus runtime_status_from_code(uint32_t code) {
    RuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

}

/// Send one document result into the runtime ingress path.
RuntimeStatus send_document_pick_result(
    uint64_t session_handle,
    uint64_t request_id,
    HostDocumentDescriptorSlice documents
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_document_result(session_handle, request_id, documents);
}
