#ifndef RUNTIME_HOST_APPLE_BRIDGE_DOCUMENT_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_DOCUMENT_RUNTIME_H

#include "../Types.h"

/// Send one document result into the runtime ingress path.
DestackRustRuntimeStatus send_document_pick_result(
    uint64_t session_handle,
    uint64_t request_id,
    DestackRustDocumentDescriptorSlice documents
);

#endif
