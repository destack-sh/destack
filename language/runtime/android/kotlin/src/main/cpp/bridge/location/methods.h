#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_LOCATION_METHODS_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_LOCATION_METHODS_H

#include "../types.h"

/// Resolve the location bridge methods from one runtime bridge instance.
bool resolve_location_methods(JNIEnv *env, jobject bridge);

/// Call the location-services-enabled entrypoint on one registered bridge.
uint32_t call_location_services_enabled(
    JNIEnv *env,
    uint64_t session_handle,
    bool *is_enabled
);

/// Call the last-known-location entrypoint on one registered bridge.
uint32_t call_location_last_known(
    JNIEnv *env,
    uint64_t session_handle,
    LocationSample *sample
);

/// Call the location-watch-open entrypoint on one registered bridge.
uint32_t call_location_watch_open(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationWatchOptions options
);

/// Call the location-watch-close entrypoint on one registered bridge.
uint32_t call_location_watch_close(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef watch_id
);

#endif
