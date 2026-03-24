#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jmethodID post_notification_method = nullptr;
jmethodID cancel_notification_method = nullptr;
jmethodID cancel_all_notifications_method = nullptr;

}

/// Resolve the notification bridge methods from one runtime bridge instance.
bool resolve_notification_methods(JNIEnv *env, jobject bridge) {
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

    if (post_notification_method == nullptr) {
        post_notification_method = env->GetMethodID(
            bridge_class,
            "postNotification",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"
        );
    }

    if (cancel_notification_method == nullptr) {
        cancel_notification_method = env->GetMethodID(
            bridge_class,
            "cancelNotification",
            "(Ljava/lang/String;)I"
        );
    }

    if (cancel_all_notifications_method == nullptr) {
        cancel_all_notifications_method = env->GetMethodID(
            bridge_class,
            "cancelAllNotifications",
            "()I"
        );
    }

    return
        post_notification_method != nullptr &&
        cancel_notification_method != nullptr &&
        cancel_all_notifications_method != nullptr;
}

/// Call the notification post entrypoint on one registered bridge.
uint32_t call_notification_post(
    JNIEnv *env,
    uint64_t session_handle,
    HostNotificationRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier = new_java_string(env, request.identifier);
    jstring title = new_java_string(env, request.title);
    jstring body = new_java_string(env, request.body);
    if (identifier == nullptr || title == nullptr || body == nullptr) {
        env->DeleteLocalRef(bridge);
        if (identifier != nullptr) {
            env->DeleteLocalRef(identifier);
        }
        if (title != nullptr) {
            env->DeleteLocalRef(title);
        }
        if (body != nullptr) {
            env->DeleteLocalRef(body);
        }
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        post_notification_method,
        identifier,
        title,
        body
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(identifier);
        env->DeleteLocalRef(title);
        env->DeleteLocalRef(body);
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(identifier);
    env->DeleteLocalRef(title);
    env->DeleteLocalRef(body);

    return static_cast<uint32_t>(status);
}

/// Call the notification cancel entrypoint on one registered bridge.
uint32_t call_notification_cancel(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef identifier
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier_string = new_java_string(env, identifier);
    if (identifier_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        cancel_notification_method,
        identifier_string
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(identifier_string);
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(identifier_string);

    return static_cast<uint32_t>(status);
}

/// Call the notification cancel-all entrypoint on one registered bridge.
uint32_t call_notification_cancel_all(JNIEnv *env, uint64_t session_handle) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jint status = env->CallIntMethod(
        bridge,
        cancel_all_notifications_method
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
