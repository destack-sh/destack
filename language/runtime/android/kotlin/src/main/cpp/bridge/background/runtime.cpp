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

}

/// Read Android background scheduler status through one attached bridge session.
uint32_t call_background_status_for_session(
    uint64_t session_handle,
    HostBackgroundStatus *output_status
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_background_status(env, session_handle, output_status);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// List Android background tasks through one attached bridge session.
uint32_t call_background_list_for_session(
    uint64_t session_handle,
    NativeArray<HostBackgroundTaskDescriptor> *output_descriptors
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_background_list(env, session_handle, output_descriptors);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Register one Android background task through one attached bridge session.
uint32_t call_background_register_for_session(
    uint64_t session_handle,
    HostBackgroundTaskOptions options
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_background_register(env, session_handle, options);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Unregister one Android background task through one attached bridge session.
uint32_t call_background_unregister_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_background_unregister(env, session_handle, identifier);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Trigger one Android background task through one attached bridge session.
uint32_t call_background_trigger_test_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    bool *is_triggered
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status =
        call_background_trigger_test(env, session_handle, identifier, is_triggered);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Complete one Android background task through one attached bridge session.
uint32_t call_background_complete_for_session(
    uint64_t session_handle,
    NativeStringRef execution_id,
    HostBackgroundTaskResult result
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status =
        call_background_complete(env, session_handle, execution_id, result);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Send one background event into the runtime ingress path.
RuntimeStatus send_background_event(
    uint64_t session_handle,
    HostBackgroundEvent event
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != HOST_STATUS_OK) {
        return status;
    }

    return bindings.notify_background_event(session_handle, event);
}

/// Deliver one background event into the runtime ingress path.
extern "C" JNIEXPORT jlongArray JNICALL
Java_dev_destack_runtime_android_bridge_background_ProcessBackgroundAbi_nativeNotifyBackgroundEvent(
    JNIEnv *env,
    jobject /* abi */,
    jlong session_handle,
    jint kind,
    jlong timestamp_ns,
    jlong sequence,
    jstring identifier,
    jstring execution_id,
    jlong deadline_unix_ns
) {
    NativeStringRef identifier_ref = native_string_ref_from_jstring(env, identifier);
    NativeStringRef execution_id_ref = native_string_ref_from_jstring(env, execution_id);
    RuntimeStatus status = send_background_event(
        static_cast<uint64_t>(session_handle),
        HostBackgroundEvent {
            .kind = static_cast<HostBackgroundEventKind>(kind),
            .metadata = {
                .timestamp_ns = static_cast<uint64_t>(timestamp_ns),
                .sequence = static_cast<uint64_t>(sequence),
                .identifier = identifier_ref,
                .execution_id = execution_id_ref,
                .deadline_unix_ns = static_cast<uint64_t>(deadline_unix_ns),
            },
        }
    );

    release_native_string_ref(env, identifier, identifier_ref);
    release_native_string_ref(env, execution_id, execution_id_ref);

    return runtime_status_array(env, status);
}
