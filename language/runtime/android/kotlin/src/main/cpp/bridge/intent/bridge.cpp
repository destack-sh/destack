#include "../types.h"
#include "../jni.h"
#include "methods.h"
#include "runtime.h"

#include <vector>

namespace {

/// Convert one JNI string into one runtime string reference.
NativeStringRef native_string_ref_from_jstring(JNIEnv *env, jstring value) {
    if (value == nullptr) {
        return {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    jsize length = env->GetStringUTFLength(value);

    return {
        .data = reinterpret_cast<const uint8_t *>(chars),
        .len = static_cast<uint32_t>(length),
    };
}

/// Release one runtime string reference borrowed from one JNI string.
void release_native_string_ref(JNIEnv *env, jstring value, NativeStringRef ref) {
    if (value == nullptr || ref.data == nullptr) {
        return;
    }

    env->ReleaseStringUTFChars(
        value,
        reinterpret_cast<const char *>(ref.data)
    );
}

/// Convert one JNI string array into runtime string references.
std::vector<NativeStringRef> native_string_refs_from_array(
    JNIEnv *env,
    jobjectArray values
) {
    std::vector<NativeStringRef> refs;

    if (values == nullptr) {
        return refs;
    }

    jsize length = env->GetArrayLength(values);
    refs.reserve(static_cast<size_t>(length));

    for (jsize index = 0; index < length; ++index) {
        jstring value = static_cast<jstring>(env->GetObjectArrayElement(values, index));

        NativeStringRef ref = native_string_ref_from_jstring(env, value);
        refs.push_back(ref);
        env->DeleteLocalRef(value);
    }

    return refs;
}

/// Release one runtime string-reference array borrowed from JNI strings.
void release_native_string_refs(
    JNIEnv *env,
    jobjectArray values,
    const std::vector<NativeStringRef> &refs
) {
    if (values == nullptr) {
        return;
    }

    jsize length = env->GetArrayLength(values);
    for (jsize index = 0; index < length; ++index) {
        jstring value = static_cast<jstring>(env->GetObjectArrayElement(values, index));

        release_native_string_ref(env, value, refs[static_cast<size_t>(index)]);
        env->DeleteLocalRef(value);
    }
}

}

/// Call one can-open-url request through the attached Kotlin bridge.
uint32_t call_can_open_url_for_session(
    uint64_t session_handle,
    NativeStringRef url,
    bool *is_supported
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_intent_can_open_url(env, session_handle, url, is_supported);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one open-url request through the attached Kotlin bridge.
uint32_t call_open_url_for_session(uint64_t session_handle, NativeStringRef url) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_intent_open_url(env, session_handle, url);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one open-path request through the attached Kotlin bridge.
uint32_t call_open_path_for_session(uint64_t session_handle, NativeStringRef path) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_intent_open_path(env, session_handle, path);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one share-text request through the attached Kotlin bridge.
uint32_t call_share_text_for_session(
    uint64_t session_handle,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_intent_share_text(
        env,
        session_handle,
        text,
        has_mime_type,
        mime_type
    );

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call one share-paths request through the attached Kotlin bridge.
uint32_t call_share_paths_for_session(
    uint64_t session_handle,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_intent_share_paths(
        env,
        session_handle,
        paths,
        has_mime_type,
        mime_type
    );

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Deliver one intent open-url event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_intent_ProcessIntentAbi_nativeNotifyIntentOpenUrl(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jboolean has_source,
    jstring source,
    jstring url
) {
    NativeStringRef source_ref = native_string_ref_from_jstring(env, source);
    NativeStringRef url_ref = native_string_ref_from_jstring(env, url);

    RuntimeStatus status = send_intent_open_url(
        static_cast<uint64_t>(session_handle),
        has_source == JNI_TRUE,
        source_ref,
        url_ref
    );

    release_native_string_ref(env, source, source_ref);
    release_native_string_ref(env, url, url_ref);

    return runtime_status_array(env, status);
}

/// Deliver one intent open-file event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_intent_ProcessIntentAbi_nativeNotifyIntentOpenFile(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jboolean has_source,
    jstring source,
    jstring path,
    jboolean has_content_type,
    jstring content_type
) {
    NativeStringRef source_ref = native_string_ref_from_jstring(env, source);
    NativeStringRef path_ref = native_string_ref_from_jstring(env, path);
    NativeStringRef content_type_ref = native_string_ref_from_jstring(env, content_type);

    RuntimeStatus status = send_intent_open_file(
        static_cast<uint64_t>(session_handle),
        has_source == JNI_TRUE,
        source_ref,
        path_ref,
        has_content_type == JNI_TRUE,
        content_type_ref
    );

    release_native_string_ref(env, source, source_ref);
    release_native_string_ref(env, path, path_ref);
    release_native_string_ref(env, content_type, content_type_ref);

    return runtime_status_array(env, status);
}

/// Deliver one intent share-text event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_intent_ProcessIntentAbi_nativeNotifyIntentShareText(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jboolean has_source,
    jstring source,
    jstring text,
    jboolean has_content_type,
    jstring content_type
) {
    NativeStringRef source_ref = native_string_ref_from_jstring(env, source);
    NativeStringRef text_ref = native_string_ref_from_jstring(env, text);
    NativeStringRef content_type_ref = native_string_ref_from_jstring(env, content_type);

    RuntimeStatus status = send_intent_share_text(
        static_cast<uint64_t>(session_handle),
        has_source == JNI_TRUE,
        source_ref,
        text_ref,
        has_content_type == JNI_TRUE,
        content_type_ref
    );

    release_native_string_ref(env, source, source_ref);
    release_native_string_ref(env, text, text_ref);
    release_native_string_ref(env, content_type, content_type_ref);

    return runtime_status_array(env, status);
}

/// Deliver one intent share-files event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_intent_ProcessIntentAbi_nativeNotifyIntentShareFiles(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jboolean has_source,
    jstring source,
    jobjectArray paths,
    jboolean has_content_type,
    jstring content_type
) {
    NativeStringRef source_ref = native_string_ref_from_jstring(env, source);
    std::vector<NativeStringRef> path_refs = native_string_refs_from_array(env, paths);
    NativeStringRef content_type_ref = native_string_ref_from_jstring(env, content_type);

    NativeStringSlice path_slice = {
        .data = path_refs.data(),
        .len = static_cast<uint32_t>(path_refs.size()),
    };

    RuntimeStatus status = send_intent_share_files(
        static_cast<uint64_t>(session_handle),
        has_source == JNI_TRUE,
        source_ref,
        path_slice,
        has_content_type == JNI_TRUE,
        content_type_ref
    );

    release_native_string_ref(env, source, source_ref);
    release_native_string_refs(env, paths, path_refs);
    release_native_string_ref(env, content_type, content_type_ref);

    return runtime_status_array(env, status);
}

/// Deliver one intent custom-action event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_intent_ProcessIntentAbi_nativeNotifyIntentCustomAction(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jboolean has_source,
    jstring source,
    jstring action,
    jboolean has_url,
    jstring url,
    jobjectArray paths,
    jboolean has_text,
    jstring text,
    jboolean has_content_type,
    jstring content_type
) {
    NativeStringRef source_ref = native_string_ref_from_jstring(env, source);
    NativeStringRef action_ref = native_string_ref_from_jstring(env, action);
    NativeStringRef url_ref = native_string_ref_from_jstring(env, url);
    std::vector<NativeStringRef> path_refs = native_string_refs_from_array(env, paths);
    NativeStringRef text_ref = native_string_ref_from_jstring(env, text);
    NativeStringRef content_type_ref = native_string_ref_from_jstring(env, content_type);

    NativeStringSlice path_slice = {
        .data = path_refs.data(),
        .len = static_cast<uint32_t>(path_refs.size()),
    };

    RuntimeStatus status = send_intent_custom_action(
        static_cast<uint64_t>(session_handle),
        has_source == JNI_TRUE,
        source_ref,
        action_ref,
        has_url == JNI_TRUE,
        url_ref,
        path_slice,
        has_text == JNI_TRUE,
        text_ref,
        has_content_type == JNI_TRUE,
        content_type_ref
    );

    release_native_string_ref(env, source, source_ref);
    release_native_string_ref(env, action, action_ref);
    release_native_string_ref(env, url, url_ref);
    release_native_string_refs(env, paths, path_refs);
    release_native_string_ref(env, text, text_ref);
    release_native_string_ref(env, content_type, content_type_ref);

    return runtime_status_array(env, status);
}
