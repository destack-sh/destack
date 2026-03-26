#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_BACKGROUND_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_BACKGROUND_METHODS_H

#include "../types.h"

/// Resolve the background bridge methods from one runtime bridge instance.
bool resolve_background_methods(JNIEnv *env, jobject bridge);
/// Call the background-status entrypoint on one registered bridge.
uint32_t call_background_status(
    JNIEnv *env,
    uint64_t session_handle,
    HostBackgroundStatus *output_status
);
/// Call the background-list entrypoint on one registered bridge.
uint32_t call_background_list(
    JNIEnv *env,
    uint64_t session_handle,
    NativeArray<HostBackgroundTaskDescriptor> *output_descriptors
);
/// Call the background-register entrypoint on one registered bridge.
uint32_t call_background_register(
    JNIEnv *env,
    uint64_t session_handle,
    HostBackgroundTaskOptions options
);
/// Call the background-unregister entrypoint on one registered bridge.
uint32_t call_background_unregister(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef identifier
);
/// Call the background-trigger entrypoint on one registered bridge.
uint32_t call_background_trigger_test(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef identifier,
    bool *is_triggered
);
/// Call the background-complete entrypoint on one registered bridge.
uint32_t call_background_complete(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef execution_id,
    HostBackgroundTaskResult result
);

#endif
