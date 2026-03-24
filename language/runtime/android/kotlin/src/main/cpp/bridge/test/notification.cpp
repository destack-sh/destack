#include "../types.h"
#include "loader.h"

#include <string>

namespace {

using SubmitTestNotificationPostFunction =
    uint32_t (*)(uint64_t, HostNotificationRequest);

SubmitTestNotificationPostFunction submit_test_notification_post = nullptr;

/// Resolve the notification testing symbols.
bool resolve_notification_testing_symbols() {
    if (submit_test_notification_post == nullptr) {
        submit_test_notification_post = reinterpret_cast<SubmitTestNotificationPostFunction>(
            resolve_testing_symbol("destack_host_android_notification_post")
        );
    }

    return submit_test_notification_post != nullptr;
}

}

/// Submit one live notification request through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeSubmitTestNotificationPost(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jstring identifier,
    jstring title,
    jstring body
) {
    if (!resolve_notification_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    const char *identifier_chars = env->GetStringUTFChars(identifier, nullptr);
    jsize identifier_length = env->GetStringUTFLength(identifier);
    const char *title_chars = env->GetStringUTFChars(title, nullptr);
    jsize title_length = env->GetStringUTFLength(title);
    const char *body_chars = env->GetStringUTFChars(body, nullptr);
    jsize body_length = env->GetStringUTFLength(body);

    HostNotificationRequest request = {
        .identifier = {
            .data = reinterpret_cast<const uint8_t *>(identifier_chars),
            .len = static_cast<uint32_t>(identifier_length),
        },
        .title = {
            .data = reinterpret_cast<const uint8_t *>(title_chars),
            .len = static_cast<uint32_t>(title_length),
        },
        .body = {
            .data = reinterpret_cast<const uint8_t *>(body_chars),
            .len = static_cast<uint32_t>(body_length),
        },
    };

    uint32_t status = submit_test_notification_post(
        static_cast<uint64_t>(session_handle),
        request
    );

    env->ReleaseStringUTFChars(identifier, identifier_chars);
    env->ReleaseStringUTFChars(title, title_chars);
    env->ReleaseStringUTFChars(body, body_chars);

    return static_cast<jint>(status);
}
