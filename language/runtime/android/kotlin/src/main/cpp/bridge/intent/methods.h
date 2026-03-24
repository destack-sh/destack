#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_INTENT_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_INTENT_METHODS_H

#include "../types.h"

/// Resolve the intent bridge methods from one runtime bridge instance.
bool resolve_intent_methods(JNIEnv *env, jobject bridge);
/// Call the can-open-url entrypoint on one registered bridge.
uint32_t call_intent_can_open_url(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef url,
    bool *is_supported
);
/// Call the open-url entrypoint on one registered bridge.
uint32_t call_intent_open_url(JNIEnv *env, uint64_t session_handle, NativeStringRef url);
/// Call the open-path entrypoint on one registered bridge.
uint32_t call_intent_open_path(JNIEnv *env, uint64_t session_handle, NativeStringRef path);
/// Call the share-text entrypoint on one registered bridge.
uint32_t call_intent_share_text(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Call the share-paths entrypoint on one registered bridge.
uint32_t call_intent_share_paths(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
);

#endif
