#include "../types.h"
#include "../jni.h"
#include "../loader.h"
#include "methods.h"
#include "runtime.h"

namespace {

/// Convert one host status code into one runtime status.
RuntimeStatus runtime_status_from_code(uint32_t code) {
    RuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Borrow one runtime string reference from one JNI string.
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

}

/// Open one Android text-input session through one attached bridge session.
uint32_t call_text_open_for_session(
    uint64_t session_handle,
    HostTextOpenRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_text_open(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Close one Android text-input session through one attached bridge session.
uint32_t call_text_close_for_session(
    uint64_t session_handle,
    uint64_t session_id
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_text_close(env, session_handle, session_id);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Update one Android text-input geometry through one attached bridge session.
uint32_t call_text_set_geometry_for_session(
    uint64_t session_handle,
    HostTextGeometryRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_text_set_geometry(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Update one Android text-input state through one attached bridge session.
uint32_t call_text_set_state_for_session(
    uint64_t session_handle,
    HostTextStateRequest request
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_text_set_state(env, session_handle, request);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Send one text-input state event into the runtime ingress path.
RuntimeStatus send_text_input_state(
    uint64_t session_handle,
    uint64_t session_id,
    HostTextSessionState state
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != HOST_STATUS_OK) {
        return status;
    }

    return bindings.notify_text_input_state(
        session_handle,
        session_id,
        state
    );
}

/// Deliver one text-input state event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_text_ProcessTextAbi_nativeNotifyTextInputState(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jlong session_id,
    jstring text,
    jint selection_start,
    jint selection_end,
    jboolean has_composing,
    jint composing_start,
    jint composing_end
) {
    NativeStringRef text_ref = native_string_ref_from_jstring(env, text);
    RuntimeStatus status = send_text_input_state(
        static_cast<uint64_t>(session_handle),
        static_cast<uint64_t>(session_id),
        HostTextSessionState {
            .text = text_ref,
            .selection = {
                .start_offset = static_cast<uint32_t>(selection_start),
                .end_offset = static_cast<uint32_t>(selection_end),
            },
            .has_composing = has_composing == JNI_TRUE,
            .composing = {
                .start_offset = static_cast<uint32_t>(composing_start),
                .end_offset = static_cast<uint32_t>(composing_end),
            },
        }
    );

    release_native_string_ref(env, text, text_ref);

    return runtime_status_array(env, status);
}
