#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

namespace {

jclass bridge_class = nullptr;
jmethodID open_text_input_method = nullptr;
jmethodID close_text_input_method = nullptr;
jmethodID set_text_input_geometry_method = nullptr;
jmethodID set_text_input_state_method = nullptr;

/// Build one JNI string borrowed from one native string reference.
jstring java_string_or_null(JNIEnv *env, NativeStringRef value) {
    if (value.data == nullptr) {
        return nullptr;
    }

    return new_java_string(env, value);
}

}

/// Resolve the text bridge methods from one runtime bridge instance.
bool resolve_text_methods(JNIEnv *env, jobject bridge) {
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

    if (open_text_input_method == nullptr) {
        open_text_input_method = env->GetMethodID(
            bridge_class,
            "openTextInput",
            "(JIZZLjava/lang/String;IIZII)I"
        );
    }

    if (close_text_input_method == nullptr) {
        close_text_input_method = env->GetMethodID(
            bridge_class,
            "closeTextInput",
            "(J)I"
        );
    }

    if (set_text_input_geometry_method == nullptr) {
        set_text_input_geometry_method = env->GetMethodID(
            bridge_class,
            "setTextInputGeometry",
            "(JDDDDDDDDDDZDDDDZDDDD)I"
        );
    }

    if (set_text_input_state_method == nullptr) {
        set_text_input_state_method = env->GetMethodID(
            bridge_class,
            "setTextInputState",
            "(JLjava/lang/String;IIZII)I"
        );
    }

    return
        open_text_input_method != nullptr &&
        close_text_input_method != nullptr &&
        set_text_input_geometry_method != nullptr &&
        set_text_input_state_method != nullptr;
}

/// Call the text-open entrypoint on one registered bridge.
uint32_t call_text_open(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextOpenRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring text = java_string_or_null(env, request.state.text);
    jint status = env->CallIntMethod(
        bridge,
        open_text_input_method,
        static_cast<jlong>(request.config.session_id),
        static_cast<jint>(request.config.input_type),
        request.config.is_multiline ? JNI_TRUE : JNI_FALSE,
        request.config.is_secure ? JNI_TRUE : JNI_FALSE,
        text,
        static_cast<jint>(request.state.selection.start_offset),
        static_cast<jint>(request.state.selection.end_offset),
        request.state.has_composing ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(request.state.composing.start_offset),
        static_cast<jint>(request.state.composing.end_offset)
    );

    if (text != nullptr) {
        env->DeleteLocalRef(text);
    }
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the text-close entrypoint on one registered bridge.
uint32_t call_text_close(
    JNIEnv *env,
    uint64_t session_handle,
    uint64_t session_id
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jint status = env->CallIntMethod(
        bridge,
        close_text_input_method,
        static_cast<jlong>(session_id)
    );

    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the text-geometry entrypoint on one registered bridge.
uint32_t call_text_set_geometry(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextGeometryRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jint status = env->CallIntMethod(
        bridge,
        set_text_input_geometry_method,
        static_cast<jlong>(request.session_id),
        static_cast<jdouble>(request.local_to_target_transform.xx),
        static_cast<jdouble>(request.local_to_target_transform.xy),
        static_cast<jdouble>(request.local_to_target_transform.yx),
        static_cast<jdouble>(request.local_to_target_transform.yy),
        static_cast<jdouble>(request.local_to_target_transform.tx),
        static_cast<jdouble>(request.local_to_target_transform.ty),
        static_cast<jdouble>(request.editor_rectangle.x),
        static_cast<jdouble>(request.editor_rectangle.y),
        static_cast<jdouble>(request.editor_rectangle.width),
        static_cast<jdouble>(request.editor_rectangle.height),
        request.has_caret_rectangle ? JNI_TRUE : JNI_FALSE,
        static_cast<jdouble>(request.caret_rectangle.x),
        static_cast<jdouble>(request.caret_rectangle.y),
        static_cast<jdouble>(request.caret_rectangle.width),
        static_cast<jdouble>(request.caret_rectangle.height),
        request.has_composing_rectangle ? JNI_TRUE : JNI_FALSE,
        static_cast<jdouble>(request.composing_rectangle.x),
        static_cast<jdouble>(request.composing_rectangle.y),
        static_cast<jdouble>(request.composing_rectangle.width),
        static_cast<jdouble>(request.composing_rectangle.height)
    );

    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}

/// Call the text-state entrypoint on one registered bridge.
uint32_t call_text_set_state(
    JNIEnv *env,
    uint64_t session_handle,
    HostTextStateRequest request
) {
    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        return HOST_STATUS_NOT_FOUND;
    }

    jstring text = java_string_or_null(env, request.state.text);
    jint status = env->CallIntMethod(
        bridge,
        set_text_input_state_method,
        static_cast<jlong>(request.session_id),
        text,
        static_cast<jint>(request.state.selection.start_offset),
        static_cast<jint>(request.state.selection.end_offset),
        request.state.has_composing ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(request.state.composing.start_offset),
        static_cast<jint>(request.state.composing.end_offset)
    );

    if (text != nullptr) {
        env->DeleteLocalRef(text);
    }
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        return HOST_STATUS_FAILED;
    }

    return static_cast<uint32_t>(status);
}
