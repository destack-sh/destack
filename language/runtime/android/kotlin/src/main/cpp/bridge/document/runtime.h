#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_DOCUMENT_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_DOCUMENT_RUNTIME_H

#include "../types.h"

/// Submit one document request through one attached bridge session.
uint32_t call_document_request_for_session(uint64_t session_handle, HostDocumentRequest request);
/// Send one document result into the runtime ingress path.
RuntimeStatus send_document_pick_result(
    uint64_t session_handle,
    uint64_t request_id,
    HostDocumentDescriptorSlice documents
);

#endif
