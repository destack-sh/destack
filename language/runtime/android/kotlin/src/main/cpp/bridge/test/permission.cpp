#include "../types.h"
#include "loader.h"

#include <string>
#include <vector>

namespace {

using SubmitTestPermissionRequestFunction =
    uint32_t (*)(uint64_t, HostPermissionRequest);
using OpenTestPermissionSettingsFunction = uint32_t (*)(uint64_t);

SubmitTestPermissionRequestFunction submit_test_permission_request = nullptr;
OpenTestPermissionSettingsFunction open_test_permission_settings = nullptr;

/// Resolve the permission testing symbols.
bool resolve_permission_testing_symbols() {
    if (submit_test_permission_request == nullptr) {
        submit_test_permission_request = reinterpret_cast<SubmitTestPermissionRequestFunction>(
            resolve_testing_symbol("destack_host_test_android_submit_permission_request")
        );
    }

    if (open_test_permission_settings == nullptr) {
        open_test_permission_settings = reinterpret_cast<OpenTestPermissionSettingsFunction>(
            resolve_testing_symbol("destack_host_test_android_open_permission_settings")
        );
    }

    return
        submit_test_permission_request != nullptr &&
        open_test_permission_settings != nullptr;
}

}

/// Read one Java string array into retained native string storage.
static NativeStringSlice read_string_array(
    JNIEnv *env,
    jobjectArray values,
    std::vector<std::string> *string_storage,
    std::vector<NativeStringRef> *ref_storage
) {
    jsize count = env->GetArrayLength(values);
    string_storage->reserve(string_storage->size() + static_cast<size_t>(count));
    ref_storage->clear();
    ref_storage->reserve(static_cast<size_t>(count));

    for (jsize index = 0; index < count; index += 1) {
        jstring value = reinterpret_cast<jstring>(env->GetObjectArrayElement(values, index));
        const char *chars = env->GetStringUTFChars(value, nullptr);
        jsize length = env->GetStringUTFLength(value);
        string_storage->emplace_back(chars, chars + length);
        env->ReleaseStringUTFChars(value, chars);
        env->DeleteLocalRef(value);

        std::string &owned = string_storage->back();
        ref_storage->push_back(NativeStringRef {
            .data = reinterpret_cast<const uint8_t *>(owned.data()),
            .len = static_cast<uint32_t>(owned.size()),
        });
    }

    return NativeStringSlice {
        .data = ref_storage->empty() ? nullptr : ref_storage->data(),
        .len = static_cast<uint32_t>(ref_storage->size()),
    };
}

/// Submit one live permission request through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeSubmitTestPermissionRequest(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jlong request_id,
    jobjectArray permissions
) {
    if (!resolve_permission_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    std::vector<std::string> permission_storage;
    std::vector<NativeStringRef> permission_refs;
    NativeStringSlice permission_slice = read_string_array(
        env,
        permissions,
        &permission_storage,
        &permission_refs
    );

    return static_cast<jint>(submit_test_permission_request(
        static_cast<uint64_t>(session_handle),
        HostPermissionRequest {
            .request_id = static_cast<uint64_t>(request_id),
            .permissions = permission_slice,
        }
    ));
}

/// Open permission settings through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeOpenTestPermissionSettings(
    JNIEnv * /* env */,
    jobject /* testing */,
    jlong session_handle
) {
    if (!resolve_permission_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    return static_cast<jint>(open_test_permission_settings(static_cast<uint64_t>(session_handle)));
}
