#include "../types.h"
#include "../jni.h"
#include "methods.h"
#include "runtime.h"

namespace {

}

/// Call one permission request through the attached Kotlin bridge.
uint32_t call_permission_request_for_session(
    uint64_t session_handle,
    HostPermissionRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_permission_request(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Open one native permission-settings surface through the attached Kotlin bridge.
uint32_t open_permission_settings_for_session(uint64_t session_handle) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_permission_settings(env, session_handle);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Deliver one permission result into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_permission_ProcessPermissionAbi_nativeNotifyPermissionResult(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle,
    jboolean has_request_id,
    jlong request_id,
    jstring permission,
    jboolean granted
) {
    const char *permission_chars = env->GetStringUTFChars(permission, nullptr);
    jsize permission_length = env->GetStringUTFLength(permission);

    NativeStringRef permission_ref = {
        .data = reinterpret_cast<const uint8_t *>(permission_chars),
        .len = static_cast<uint32_t>(permission_length),
    };

    RuntimeStatus status = send_permission_result(
        static_cast<uint64_t>(session_handle),
        has_request_id == JNI_TRUE,
        static_cast<uint64_t>(request_id),
        permission_ref,
        granted == JNI_TRUE
    );

    env->ReleaseStringUTFChars(permission, permission_chars);

    return runtime_status_array(env, status);
}
