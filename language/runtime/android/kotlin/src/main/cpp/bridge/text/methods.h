#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TEXT_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TEXT_METHODS_H

#include "../types.h"

/// Resolve the text bridge methods from one runtime bridge instance.
bool resolve_text_methods(JNIEnv *env, jobject bridge);
/// Call the text-open entrypoint on one registered bridge.
uint32_t call_text_open(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextOpenRequest request
);
/// Call the text-close entrypoint on one registered bridge.
uint32_t call_text_close(
    JNIEnv *env,
    uint64_t session_handle,
    uint64_t session_id
);
/// Call the text-geometry entrypoint on one registered bridge.
uint32_t call_text_set_geometry(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextGeometryRequest request
);
/// Call the text-state entrypoint on one registered bridge.
uint32_t call_text_set_state(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextStateRequest request
);

#endif
