#include "../types.h"
#include "../jni.h"
#include "methods.h"
#include "runtime.h"

#include <vector>

namespace {

jclass document_descriptor_class = nullptr;
jclass long_class = nullptr;
jmethodID get_uri_method = nullptr;
jmethodID get_name_method = nullptr;
jmethodID get_mime_type_method = nullptr;
jmethodID get_size_bytes_method = nullptr;
jmethodID get_modified_unix_ns_method = nullptr;
jmethodID is_directory_method = nullptr;
jmethodID get_local_path_method = nullptr;
jmethodID long_value_method = nullptr;

/// Resolve the descriptor accessors for one document result payload.
bool resolve_document_descriptor_methods(JNIEnv *env, jobjectArray documents) {
    if (document_descriptor_class == nullptr) {
        jobject first_value = env->GetObjectArrayElement(documents, 0);
        if (first_value == nullptr) {
            return false;
        }

        jclass local_class = env->GetObjectClass(first_value);
        env->DeleteLocalRef(first_value);
        if (local_class == nullptr) {
            return false;
        }

        document_descriptor_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (document_descriptor_class == nullptr) {
            return false;
        }
    }

    if (long_class == nullptr) {
        jclass local_long_class = env->FindClass("java/lang/Long");
        if (local_long_class == nullptr) {
            return false;
        }

        long_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_long_class));
        env->DeleteLocalRef(local_long_class);
        if (long_class == nullptr) {
            return false;
        }
    }

    if (get_uri_method == nullptr) {
        get_uri_method = env->GetMethodID(
            document_descriptor_class,
            "getUri",
            "()Ljava/lang/String;"
        );
    }

    if (get_name_method == nullptr) {
        get_name_method = env->GetMethodID(
            document_descriptor_class,
            "getName",
            "()Ljava/lang/String;"
        );
    }

    if (get_mime_type_method == nullptr) {
        get_mime_type_method = env->GetMethodID(
            document_descriptor_class,
            "getMimeType",
            "()Ljava/lang/String;"
        );
    }

    if (get_size_bytes_method == nullptr) {
        get_size_bytes_method = env->GetMethodID(
            document_descriptor_class,
            "getSizeBytes",
            "()Ljava/lang/Long;"
        );
    }

    if (get_modified_unix_ns_method == nullptr) {
        get_modified_unix_ns_method = env->GetMethodID(
            document_descriptor_class,
            "getModifiedUnixNs",
            "()Ljava/lang/Long;"
        );
    }

    if (is_directory_method == nullptr) {
        is_directory_method = env->GetMethodID(
            document_descriptor_class,
            "isDirectory",
            "()Z"
        );
    }

    if (get_local_path_method == nullptr) {
        get_local_path_method = env->GetMethodID(
            document_descriptor_class,
            "getLocalPath",
            "()Ljava/lang/String;"
        );
    }

    if (long_value_method == nullptr) {
        long_value_method = env->GetMethodID(long_class, "longValue", "()J");
    }

    return
        get_uri_method != nullptr &&
        get_name_method != nullptr &&
        get_mime_type_method != nullptr &&
        get_size_bytes_method != nullptr &&
        get_modified_unix_ns_method != nullptr &&
        is_directory_method != nullptr &&
        get_local_path_method != nullptr &&
        long_value_method != nullptr;
}

/// Convert one Java string into one native string reference backed by local storage.
NativeStringRef string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::vector<std::string> *storage
) {
    if (value == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    jsize length = env->GetStringUTFLength(value);
    storage->emplace_back(chars, chars + length);
    env->ReleaseStringUTFChars(value, chars);
    std::string &owned = storage->back();

    return NativeStringRef {
        .data = reinterpret_cast<const uint8_t *>(owned.data()),
        .len = static_cast<uint32_t>(owned.size()),
    };
}

}

/// Call one document request through the attached Kotlin bridge.
uint32_t call_document_request_for_session(
    uint64_t session_handle,
    HostDocumentRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_document_request(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Deliver one document result into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_document_ProcessDocumentAbi_nativeNotifyDocumentResult(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle,
    jlong request_id,
    jobjectArray documents
) {
    jsize count = env->GetArrayLength(documents);
    std::vector<std::string> string_storage;
    std::vector<HostDocumentDescriptor> descriptor_storage;
    string_storage.reserve(static_cast<size_t>(count) * 4);
    descriptor_storage.reserve(static_cast<size_t>(count));

    if (count != 0 && !resolve_document_descriptor_methods(env, documents)) {
        RuntimeStatus status = {
            .code = HOST_STATUS_FAILED,
            .error_id = 0,
        };

        return runtime_status_array(env, status);
    }

    for (jsize index = 0; index < count; index += 1) {
        jobject document = env->GetObjectArrayElement(documents, index);
        if (document == nullptr) {
            RuntimeStatus status = {
                .code = HOST_STATUS_FAILED,
                .error_id = 0,
            };

            return runtime_status_array(env, status);
        }

        jstring uri = reinterpret_cast<jstring>(env->CallObjectMethod(document, get_uri_method));
        jstring name = reinterpret_cast<jstring>(env->CallObjectMethod(document, get_name_method));
        jstring mime_type = reinterpret_cast<jstring>(env->CallObjectMethod(document, get_mime_type_method));
        jobject size_bytes = env->CallObjectMethod(document, get_size_bytes_method);
        jobject modified_unix_ns = env->CallObjectMethod(document, get_modified_unix_ns_method);
        jboolean is_directory = env->CallBooleanMethod(document, is_directory_method);
        jstring local_path = reinterpret_cast<jstring>(env->CallObjectMethod(document, get_local_path_method));

        HostDocumentDescriptor descriptor = {
            .uri = string_ref_from_java(env, uri, &string_storage),
            .name = string_ref_from_java(env, name, &string_storage),
            .has_mime_type = mime_type != nullptr,
            .mime_type = string_ref_from_java(env, mime_type, &string_storage),
            .has_size_bytes = size_bytes != nullptr,
            .size_bytes = size_bytes == nullptr ? 0 : static_cast<uint64_t>(
                env->CallLongMethod(size_bytes, long_value_method)
            ),
            .has_modified_unix_ns = modified_unix_ns != nullptr,
            .modified_unix_ns = modified_unix_ns == nullptr ? 0 : static_cast<uint64_t>(
                env->CallLongMethod(modified_unix_ns, long_value_method)
            ),
            .is_directory = is_directory == JNI_TRUE,
            .has_local_path = local_path != nullptr,
            .local_path = string_ref_from_java(env, local_path, &string_storage),
        };
        descriptor_storage.push_back(descriptor);

        env->DeleteLocalRef(document);
        if (uri != nullptr) env->DeleteLocalRef(uri);
        if (name != nullptr) env->DeleteLocalRef(name);
        if (mime_type != nullptr) env->DeleteLocalRef(mime_type);
        if (size_bytes != nullptr) env->DeleteLocalRef(size_bytes);
        if (modified_unix_ns != nullptr) env->DeleteLocalRef(modified_unix_ns);
        if (local_path != nullptr) env->DeleteLocalRef(local_path);
    }

    HostDocumentDescriptorSlice descriptor_slice = {
        .data = descriptor_storage.empty() ? nullptr : descriptor_storage.data(),
        .len = static_cast<uint32_t>(descriptor_storage.size()),
    };

    RuntimeStatus status = send_document_pick_result(
        static_cast<uint64_t>(session_handle),
        static_cast<uint64_t>(request_id),
        descriptor_slice
    );

    return runtime_status_array(env, status);
}
