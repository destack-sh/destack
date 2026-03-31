#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_JNI_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_JNI_H

#include "types.h"

/// Resolve one JNI environment and report whether this call attached the thread.
bool resolve_jni_env(JNIEnv **out_env, bool *out_did_attach_thread);
/// Detach one JNI thread when this callback attached it.
void detach_jni_thread(bool did_attach_thread);
/// Build one Java string from one native string reference.
jstring new_java_string(JNIEnv *env, NativeStringRef value);
/// Build one Java string array from one native string slice.
jobjectArray new_java_string_array(JNIEnv *env, NativeStringSlice values);
/// Register one native method table on one JVM class.
bool register_native_methods(
    JNIEnv *env,
    const char *class_name,
    JNINativeMethod *methods,
    jint count
);
/// Call one bridge method with one byte payload.
uint32_t call_bridge_bytes(
    JNIEnv *env,
    uint64_t session_handle,
    NativeSlice payload,
    jmethodID method
);
/// Encode one runtime status into one long array for JNI.
jlongArray runtime_status_array(JNIEnv *env, RuntimeStatus status);

#endif
