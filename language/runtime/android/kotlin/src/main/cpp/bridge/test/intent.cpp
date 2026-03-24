#include "../types.h"
#include "loader.h"

namespace {

using OpenTestIntentCanOpenUrlFunction = uint32_t (*)(uint64_t, NativeStringRef, bool *);

OpenTestIntentCanOpenUrlFunction open_test_intent_can_open_url = nullptr;

/// Resolve the intent testing symbols.
bool resolve_intent_testing_symbols() {
    if (open_test_intent_can_open_url == nullptr) {
        open_test_intent_can_open_url = reinterpret_cast<OpenTestIntentCanOpenUrlFunction>(
            resolve_testing_symbol("destack_host_test_android_intent_can_open_url")
        );
    }

    return open_test_intent_can_open_url != nullptr;
}

}

/// Query can-open-url through the Android bridge test harness.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeTestIntentCanOpenUrl(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jstring url
) {
    if (!resolve_intent_testing_symbols()) {
        return nullptr;
    }

    const char *url_chars = env->GetStringUTFChars(url, nullptr);
    jsize url_length = env->GetStringUTFLength(url);
    bool is_supported = false;
    uint32_t status = open_test_intent_can_open_url(
        static_cast<uint64_t>(session_handle),
        NativeStringRef {
            .data = reinterpret_cast<const uint8_t *>(url_chars),
            .len = static_cast<uint32_t>(url_length),
        },
        &is_supported
    );

    env->ReleaseStringUTFChars(url, url_chars);

    jlong values[2] = {
        static_cast<jlong>(status),
        static_cast<jlong>(is_supported ? 1 : 0),
    };
    jlongArray result = env->NewLongArray(2);
    if (result == nullptr) {
        return nullptr;
    }

    env->SetLongArrayRegion(result, 0, 2, values);

    return result;
}
