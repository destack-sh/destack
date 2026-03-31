#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jmethodID submit_document_request_method = nullptr;

}

/// Resolve the document bridge methods from one runtime bridge instance.
bool resolve_document_methods(JNIEnv *env, jobject bridge) {
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

    if (submit_document_request_method == nullptr) {
        submit_document_request_method = env->GetMethodID(
            bridge_class,
            "documentPick",
            "(J[Ljava/lang/String;[Ljava/lang/String;ZZZ)I"
        );
    }

    return submit_document_request_method != nullptr;
}

/// Call the document request entrypoint on one registered bridge.
uint32_t call_document_request(
    JNIEnv *env,
    uint64_t session_handle,
    HostDocumentRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobjectArray mime_types = new_java_string_array(env, request.mime_types);
    if (mime_types == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jobjectArray extensions = new_java_string_array(env, request.extensions);
    if (extensions == nullptr) {
        env->DeleteLocalRef(bridge);
        env->DeleteLocalRef(mime_types);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        submit_document_request_method,
        static_cast<jlong>(request.request_id),
        mime_types,
        extensions,
        request.allows_multiple_selection ? JNI_TRUE : JNI_FALSE,
        request.allows_directory_selection ? JNI_TRUE : JNI_FALSE,
        request.copies_to_sandbox ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(mime_types);
    env->DeleteLocalRef(extensions);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
