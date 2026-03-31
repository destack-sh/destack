#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jmethodID submit_permission_request_method = nullptr;
jmethodID open_permission_settings_method = nullptr;

}

/// Resolve the permission bridge methods from one runtime bridge instance.
bool resolve_permission_methods(JNIEnv *env, jobject bridge) {
    if (bridge_class == nullptr) {
        jclass local_bridge_class = env->GetObjectClass(bridge);
        if (local_bridge_class == nullptr) {
            return false;
        }

        bridge_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_bridge_class));
        env->DeleteLocalRef(local_bridge_class);
        if (bridge_class == nullptr) {
            return false;
        }
    }

    if (submit_permission_request_method == nullptr) {
        submit_permission_request_method = env->GetMethodID(
            bridge_class,
            "permissionRequest",
            "(J[Ljava/lang/String;)I"
        );
    }

    if (open_permission_settings_method == nullptr) {
        open_permission_settings_method = env->GetMethodID(
            bridge_class,
            "permissionOpenSettings",
            "()I"
        );
    }

    return
        submit_permission_request_method != nullptr &&
        open_permission_settings_method != nullptr;
}

/// Call the permission request entrypoint on one registered bridge.
uint32_t call_permission_request(
    JNIEnv *env,
    uint64_t session_handle,
    HostPermissionRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobjectArray permissions = new_java_string_array(env, request.permissions);
    if (permissions == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        submit_permission_request_method,
        static_cast<jlong>(request.request_id),
        permissions
    );
    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(permissions);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the permission-settings entrypoint on one registered bridge.
uint32_t call_permission_settings(JNIEnv *env, uint64_t session_handle) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jint status = env->CallIntMethod(bridge, open_permission_settings_method);
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
