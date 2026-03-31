#include "types.h"
#include "jni.h"
#include "background/callbacks.generated.h"
#include "background/runtime.generated.h"
#include "calendar/callbacks.generated.h"
#include "calendar/runtime.generated.h"
#include "contact/callbacks.generated.h"
#include "contact/runtime.generated.h"
#include "document/callbacks.generated.h"
#include "document/runtime.generated.h"
#include "intent/callbacks.generated.h"
#include "intent/runtime.generated.h"
#include "location/callbacks.generated.h"
#include "location/runtime.generated.h"
#include "media/callbacks.generated.h"
#include "media/runtime.generated.h"
#include "notification/callbacks.generated.h"
#include "notification/runtime.generated.h"
#include "permission/callbacks.generated.h"
#include "permission/runtime.generated.h"
#include "text/callbacks.generated.h"
#include "text/runtime.generated.h"
#include "runtime_abi.generated.h"
#include "registry.h"

#include <mutex>
#include <unordered_map>

namespace {

std::mutex bridge_lock;
std::unordered_map<uint64_t, jobject> bridges;

/// Resolve the runtime bridge methods from one generated bridge instance.
bool resolve_runtime_bridge_methods(JNIEnv *env, jobject bridge) {
    return
        resolve_background_methods(env, bridge) &&
        resolve_calendar_methods(env, bridge) &&
        resolve_contact_methods(env, bridge) &&
        resolve_document_methods(env, bridge) &&
        resolve_intent_methods(env, bridge) &&
        resolve_location_methods(env, bridge) &&
        resolve_media_methods(env, bridge) &&
        resolve_notification_methods(env, bridge) &&
        resolve_permission_methods(env, bridge) &&
        resolve_text_methods(env, bridge);
}

/// Register the runtime ingress JNI methods.
bool register_runtime_ingress_natives_impl(JNIEnv *env) {
    return
        register_background_runtime_natives(env) &&
        register_document_runtime_natives(env) &&
        register_intent_runtime_natives(env) &&
        register_location_runtime_natives(env) &&
        register_notification_runtime_natives(env) &&
        register_permission_runtime_natives(env) &&
        register_text_runtime_natives(env);
}

/// Build one runtime bridge callback table for the generated Android bridge.
AndroidRuntimeBridgeBindings make_runtime_bridge_bindings() {
    return AndroidRuntimeBridgeBindings {
        .document = make_document_callbacks(),
        .permission = make_permission_callbacks(),
        .calendar = make_calendar_callbacks(),
        .contact = make_contact_callbacks(),
        .intent = make_intent_callbacks(),
        .location = make_location_callbacks(),
        .media = make_media_callbacks(),
        .notification = make_notification_callbacks(),
        .text = make_text_callbacks(),
        .background = make_background_callbacks(),
    };
}

}

/// Resolve one registered bridge for one runtime session.
jobject resolve_bridge(JNIEnv *env, uint64_t session_handle) {
    std::lock_guard<std::mutex> guard(bridge_lock);

    auto it = bridges.find(session_handle);
    if (it == bridges.end()) {
        return nullptr;
    }

    return env->NewLocalRef(it->second);
}

/// Attach one bridge instance to one runtime session.
bool attach_bridge(JNIEnv *env, uint64_t session_handle, jobject bridge) {
    jobject global_bridge = env->NewGlobalRef(bridge);
    if (global_bridge == nullptr) {
        return false;
    }

    std::lock_guard<std::mutex> guard(bridge_lock);
    auto it = bridges.find(session_handle);
    if (it != bridges.end()) {
        env->DeleteGlobalRef(it->second);
    }

    bridges[session_handle] = global_bridge;

    return true;
}

/// Detach one bridge instance from one runtime session.
void detach_bridge(JNIEnv *env, uint64_t session_handle) {
    std::lock_guard<std::mutex> guard(bridge_lock);

    auto it = bridges.find(session_handle);
    if (it == bridges.end()) {
        return;
    }

    env->DeleteGlobalRef(it->second);
    bridges.erase(it);
}

/// Attach one bridge instance to one runtime session through JNI.
static jint nativeAttachBridge(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle,
    jobject bridge
) {
    if (!resolve_runtime_bridge_methods(env, bridge)) {
        return static_cast<jint>(HOST_STATUS_FAILED);
    }

    bool did_attach = attach_bridge(env, static_cast<uint64_t>(session_handle), bridge);
    if (!did_attach) {
        return static_cast<jint>(HOST_STATUS_FAILED);
    }

    AndroidRuntimeBridgeBindings bindings = make_runtime_bridge_bindings();

    uint32_t status = destack_host_android_register_runtime_bridge_bindings(
        static_cast<uint64_t>(session_handle),
        bindings
    );
    if (status != HOST_STATUS_OK) {
        detach_bridge(env, static_cast<uint64_t>(session_handle));
        return static_cast<jint>(status);
    }

    return static_cast<jint>(status);
}

/// Detach one bridge instance from one runtime session through JNI.
static void nativeDetachBridge(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle
) {
    destack_host_android_unregister_runtime_bridge_bindings(
        static_cast<uint64_t>(session_handle)
    );

    detach_bridge(env, static_cast<uint64_t>(session_handle));
}

/// Register the runtime ingress JNI methods.
bool register_runtime_ingress_natives(JNIEnv *env) {
    return register_runtime_ingress_natives_impl(env);
}

/// Register the runtime session JNI methods.
bool register_runtime_session_natives(JNIEnv *env) {
    JNINativeMethod methods[] = {
        {
            const_cast<char *>("nativeAttachBridge"),
            const_cast<char *>("(JLdev/destack/runtime/android/bridge/RuntimeBridge;)I"),
            reinterpret_cast<void *>(nativeAttachBridge),
        },
        {
            const_cast<char *>("nativeDetachBridge"),
            const_cast<char *>("(J)V"),
            reinterpret_cast<void *>(nativeDetachBridge),
        },
    };

    return register_native_methods(
        env,
        "dev/destack/runtime/android/bridge/ProcessRuntimeIngress",
        methods,
        static_cast<jint>(sizeof(methods) / sizeof(methods[0]))
    );
}
