#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_DOCUMENT_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_DOCUMENT_METHODS_H

#include "../types.h"

/// Resolve the document bridge methods from one runtime bridge instance.
bool resolve_document_methods(JNIEnv *env, jobject bridge);
/// Call the document request entrypoint on one registered bridge.
uint32_t call_document_request(
    JNIEnv *env,
    uint64_t session_handle,
    HostDocumentRequest request
);

#endif
