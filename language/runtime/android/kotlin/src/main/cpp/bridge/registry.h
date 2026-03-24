#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_REGISTRY_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_REGISTRY_H

#include "types.h"

/// Resolve one registered bridge for one runtime session.
jobject resolve_bridge(JNIEnv *env, uint64_t session_handle);
/// Attach one bridge instance to one runtime session.
bool attach_bridge(JNIEnv *env, uint64_t session_handle, jobject bridge);
/// Detach one bridge instance from one runtime session.
void detach_bridge(JNIEnv *env, uint64_t session_handle);

#endif
