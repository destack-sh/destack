#include "../types.h"
#include "loader.h"

#include <string>
#include <vector>

namespace {

using SubmitTestDocumentRequestFunction =
    uint32_t (*)(uint64_t, HostDocumentRequest);

SubmitTestDocumentRequestFunction submit_test_document_request = nullptr;

/// Resolve the document testing symbols.
bool resolve_document_testing_symbols() {
    if (submit_test_document_request == nullptr) {
        submit_test_document_request = reinterpret_cast<SubmitTestDocumentRequestFunction>(
            resolve_testing_symbol("destack_host_test_android_submit_document_request")
        );
    }

    return submit_test_document_request != nullptr;
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

/// Submit one live document request through the Android bridge test harness.
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeSubmitTestDocumentRequest(
    JNIEnv *env,
    jobject /* testing */,
    jlong session_handle,
    jlong request_id,
    jobjectArray mime_types,
    jobjectArray extensions,
    jboolean allows_multiple_selection,
    jboolean allows_directory_selection,
    jboolean copies_to_sandbox
) {
    if (!resolve_document_testing_symbols()) {
        return static_cast<jint>(HOST_STATUS_NOT_FOUND);
    }

    std::vector<std::string> mime_type_storage;
    std::vector<NativeStringRef> mime_type_refs;
    NativeStringSlice mime_type_slice = read_string_array(
        env,
        mime_types,
        &mime_type_storage,
        &mime_type_refs
    );
    std::vector<std::string> extension_storage;
    std::vector<NativeStringRef> extension_refs;
    NativeStringSlice extension_slice = read_string_array(
        env,
        extensions,
        &extension_storage,
        &extension_refs
    );

    return static_cast<jint>(submit_test_document_request(
        static_cast<uint64_t>(session_handle),
        HostDocumentRequest {
            .request_id = static_cast<uint64_t>(request_id),
            .mime_types = mime_type_slice,
            .extensions = extension_slice,
            .allows_multiple_selection = allows_multiple_selection == JNI_TRUE,
            .allows_directory_selection = allows_directory_selection == JNI_TRUE,
            .copies_to_sandbox = copies_to_sandbox == JNI_TRUE,
        }
    ));
}
