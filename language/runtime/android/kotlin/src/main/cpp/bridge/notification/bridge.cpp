#include "../types.h"
#include "../jni.h"
#include "methods.h"
#include "runtime.h"

#include <string>

namespace {

/// Build one borrowed native string reference from one Java string.
NativeStringRef string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::string *storage
) {
    if (value == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    jsize length = env->GetStringUTFLength(value);
    *storage = std::string(chars, chars + length);
    env->ReleaseStringUTFChars(value, chars);

    return NativeStringRef {
        .data = reinterpret_cast<const uint8_t *>(storage->data()),
        .len = static_cast<uint32_t>(storage->size()),
    };
}

}

/// Call one notification post request through the attached Kotlin bridge.
uint32_t call_notification_post_for_session(
    uint64_t session_handle,
    HostNotificationRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_notification_post(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one notification cancel request through the attached Kotlin bridge.
uint32_t call_notification_cancel_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_notification_cancel(env, session_handle, identifier);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one notification cancel-all request through the attached Kotlin bridge.
uint32_t call_notification_cancel_all_for_session(uint64_t session_handle) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_notification_cancel_all(env, session_handle);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Deliver one notification event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_notification_ProcessNotificationAbi_nativeNotifyNotificationEvent(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle,
    jint kind,
    jlong sequence,
    jlong timestamp_ns,
    jstring identifier,
    jstring title,
    jstring body,
    jstring action_identifier
) {
    std::string identifier_storage;
    std::string title_storage;
    std::string body_storage;
    std::string action_identifier_storage;

    HostNotificationEvent event = {
        .kind = static_cast<HostNotificationEventKind>(kind),
        .sequence = static_cast<uint64_t>(sequence),
        .timestamp_ns = static_cast<uint64_t>(timestamp_ns),
        .request = {
            .identifier = string_ref_from_java(env, identifier, &identifier_storage),
            .title = string_ref_from_java(env, title, &title_storage),
            .body = string_ref_from_java(env, body, &body_storage),
        },
        .has_action_identifier = action_identifier != nullptr,
        .action_identifier = string_ref_from_java(
            env,
            action_identifier,
            &action_identifier_storage
        ),
    };

    RuntimeStatus status = send_notification_event(
        static_cast<uint64_t>(session_handle),
        event
    );

    return runtime_status_array(env, status);
}
