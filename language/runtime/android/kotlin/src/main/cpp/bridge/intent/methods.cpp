#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jmethodID can_open_url_method = nullptr;
jmethodID open_url_method = nullptr;
jmethodID open_path_method = nullptr;
jmethodID share_text_method = nullptr;
jmethodID share_paths_method = nullptr;

}

/// Resolve the intent bridge methods from one runtime bridge instance.
bool resolve_intent_methods(JNIEnv *env, jobject bridge) {
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

    if (can_open_url_method == nullptr) {
        can_open_url_method = env->GetMethodID(
            bridge_class,
            "canOpenUrl",
            "(Ljava/lang/String;)Z"
        );
    }

    if (open_url_method == nullptr) {
        open_url_method = env->GetMethodID(
            bridge_class,
            "openUrl",
            "(Ljava/lang/String;)I"
        );
    }

    if (open_path_method == nullptr) {
        open_path_method = env->GetMethodID(
            bridge_class,
            "openPath",
            "(Ljava/lang/String;)I"
        );
    }

    if (share_text_method == nullptr) {
        share_text_method = env->GetMethodID(
            bridge_class,
            "shareText",
            "(Ljava/lang/String;Ljava/lang/String;)I"
        );
    }

    if (share_paths_method == nullptr) {
        share_paths_method = env->GetMethodID(
            bridge_class,
            "sharePaths",
            "([Ljava/lang/String;Ljava/lang/String;)I"
        );
    }

    return
        can_open_url_method != nullptr &&
        open_url_method != nullptr &&
        open_path_method != nullptr &&
        share_text_method != nullptr &&
        share_paths_method != nullptr;
}

/// Call the can-open-url entrypoint on one registered bridge.
uint32_t call_intent_can_open_url(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef url,
    bool *is_supported
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring url_string = new_java_string(env, url);
    if (url_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jboolean supported = env->CallBooleanMethod(bridge, can_open_url_method, url_string);
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(url_string);
        return HOST_STATUS_FAILED;
    }

    *is_supported = supported == JNI_TRUE;
    env->DeleteLocalRef(url_string);

    return 0;
}

/// Call the open-url entrypoint on one registered bridge.
uint32_t call_intent_open_url(JNIEnv *env, uint64_t session_handle, NativeStringRef url) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring url_string = new_java_string(env, url);
    if (url_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(bridge, open_url_method, url_string);
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(url_string);
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(url_string);

    return static_cast<uint32_t>(status);
}

/// Call the open-path entrypoint on one registered bridge.
uint32_t call_intent_open_path(JNIEnv *env, uint64_t session_handle, NativeStringRef path) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring path_string = new_java_string(env, path);
    if (path_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(bridge, open_path_method, path_string);
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(path_string);
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(path_string);

    return static_cast<uint32_t>(status);
}

/// Call the share-text entrypoint on one registered bridge.
uint32_t call_intent_share_text(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring text_string = new_java_string(env, text);
    if (text_string == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jstring mime_type_string = has_mime_type
        ? new_java_string(env, mime_type)
        : nullptr;
    if (has_mime_type && mime_type_string == nullptr) {
        env->DeleteLocalRef(bridge);
        env->DeleteLocalRef(text_string);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        share_text_method,
        text_string,
        mime_type_string
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(text_string);
        if (mime_type_string != nullptr) {
            env->DeleteLocalRef(mime_type_string);
        }
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(text_string);
    if (mime_type_string != nullptr) {
        env->DeleteLocalRef(mime_type_string);
    }

    return static_cast<uint32_t>(status);
}

/// Call the share-paths entrypoint on one registered bridge.
uint32_t call_intent_share_paths(
    JNIEnv *env,
    uint64_t session_handle,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jobjectArray path_array = new_java_string_array(env, paths);
    if (path_array == nullptr) {
        env->DeleteLocalRef(bridge);
        return HOST_STATUS_FAILED;
    }

    jstring mime_type_string = has_mime_type
        ? new_java_string(env, mime_type)
        : nullptr;
    if (has_mime_type && mime_type_string == nullptr) {
        env->DeleteLocalRef(bridge);
        env->DeleteLocalRef(path_array);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        share_paths_method,
        path_array,
        mime_type_string
    );
    env->DeleteLocalRef(bridge);
    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        env->DeleteLocalRef(path_array);
        if (mime_type_string != nullptr) {
            env->DeleteLocalRef(mime_type_string);
        }
        return HOST_STATUS_FAILED;
    }

    env->DeleteLocalRef(path_array);
    if (mime_type_string != nullptr) {
        env->DeleteLocalRef(mime_type_string);
    }

    return static_cast<uint32_t>(status);
}
