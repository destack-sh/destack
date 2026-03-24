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

/// Read whether Android location services are enabled through one attached bridge session.
uint32_t call_location_services_enabled_for_session(
    uint64_t session_handle,
    bool *is_enabled
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_location_services_enabled(env, session_handle, is_enabled);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Read one Android last-known location sample through one attached bridge session.
uint32_t call_location_last_known_for_session(
    uint64_t session_handle,
    LocationSample *sample
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_location_last_known(env, session_handle, sample);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Open one Android location watch through one attached bridge session.
uint32_t call_location_watch_open_for_session(
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationWatchOptions options
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_location_watch_open(env, session_handle, watch_id, options);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Close one Android location watch through one attached bridge session.
uint32_t call_location_watch_close_for_session(
    uint64_t session_handle,
    NativeStringRef watch_id
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    uint32_t status = call_location_watch_close(env, session_handle, watch_id);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Send one location sample into the runtime ingress path.
RuntimeStatus send_location_sample(
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationSample sample
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_location_sample(session_handle, watch_id, sample);
}
