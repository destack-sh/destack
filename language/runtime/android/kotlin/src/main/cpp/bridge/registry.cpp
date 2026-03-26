#include "types.h"
#include "calendar/methods.h"
#include "contact/methods.h"
#include "document/runtime.h"
#include "document/methods.h"
#include "intent/runtime.h"
#include "intent/methods.h"
#include "location/runtime.h"
#include "location/methods.h"
#include "media/methods.h"
#include "notification/methods.h"
#include "notification/runtime.h"
#include "permission/methods.h"
#include "permission/runtime.h"
#include "text/methods.h"
#include "text/runtime.h"
#include "loader.h"
#include "registry.h"

#include <mutex>
#include <unordered_map>

namespace {

std::mutex bridge_lock;
std::unordered_map<uint64_t, jobject> bridges;

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
extern "C" JNIEXPORT jint JNICALL
Java_dev_destack_runtime_android_bridge_ProcessRuntimeAbi_nativeAttachBridge(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle,
    jobject bridge
) {
    bool has_document_methods = resolve_document_methods(env, bridge);
    bool has_permission_methods = resolve_permission_methods(env, bridge);
    bool has_calendar_methods = resolve_calendar_methods(env, bridge);
    bool has_contact_methods = resolve_contact_methods(env, bridge);
    bool has_intent_methods = resolve_intent_methods(env, bridge);
    bool has_location_methods = resolve_location_methods(env, bridge);
    bool has_media_methods = resolve_media_methods(env, bridge);
    bool has_notification_methods = resolve_notification_methods(env, bridge);
    bool has_text_methods = resolve_text_methods(env, bridge);

    if (!(has_document_methods && has_permission_methods && has_calendar_methods &&
            has_contact_methods &&
            has_intent_methods &&
            has_location_methods && has_media_methods &&
            has_text_methods &&
            has_notification_methods)) {
        return static_cast<jint>(HOST_STATUS_FAILED);
    }

    bool did_attach = attach_bridge(env, static_cast<uint64_t>(session_handle), bridge);
    if (!did_attach) {
        return static_cast<jint>(HOST_STATUS_FAILED);
    }

    AndroidRuntimeBridgeBindings bindings = {
        .document = {
            .pick = call_document_request_for_session,
        },
        .permission = {
            .open_settings = open_permission_settings_for_session,
            .request = call_permission_request_for_session,
        },
        .calendar = {
            .list = call_calendar_list_for_session,
            .event_list = call_calendar_event_list_for_session,
            .event_read = call_calendar_event_read_for_session,
            .event_create = call_calendar_event_create_for_session,
            .event_update = call_calendar_event_update_for_session,
            .event_delete = call_calendar_event_delete_for_session,
        },
        .contact = {
            .list = call_contact_list_for_session,
            .search = call_contact_search_for_session,
            .read = call_contact_read_for_session,
            .create = call_contact_create_for_session,
            .update = call_contact_update_for_session,
            .delete_contact = call_contact_delete_for_session,
        },
        .intent = {
            .can_open_url = call_can_open_url_for_session,
            .open_url = call_open_url_for_session,
            .open_path = call_open_path_for_session,
            .share_text = call_share_text_for_session,
            .share_paths = call_share_paths_for_session,
        },
        .location = {
            .services_enabled = call_location_services_enabled_for_session,
            .last_known = call_location_last_known_for_session,
            .watch_open = call_location_watch_open_for_session,
            .watch_close = call_location_watch_close_for_session,
        },
        .media = {
            .list = call_media_list_for_session,
            .describe = call_media_read_for_session,
            .import_path = call_media_import_path_for_session,
            .delete_media = call_media_delete_for_session,
        },
        .notification = {
            .cancel = call_notification_cancel_for_session,
            .cancel_all = call_notification_cancel_all_for_session,
            .post = call_notification_post_for_session,
        },
        .text = {
            .open = call_text_open_for_session,
            .close = call_text_close_for_session,
            .set_geometry = call_text_set_geometry_for_session,
            .set_state = call_text_set_state_for_session,
        },
    };

    RuntimeBindings runtime_bindings = {};
    uint32_t status = resolve_runtime_bindings(&runtime_bindings);
    if (status != HOST_STATUS_OK) {
        detach_bridge(env, static_cast<uint64_t>(session_handle));
        return static_cast<jint>(status);
    }

    status = runtime_bindings.register_runtime_bridge_bindings(
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
extern "C" JNIEXPORT void JNICALL
Java_dev_destack_runtime_android_bridge_ProcessRuntimeAbi_nativeDetachBridge(
    JNIEnv *env,
    jobject /* bindings */,
    jlong session_handle
) {
    RuntimeBindings runtime_bindings = {};
    if (resolve_runtime_bindings(&runtime_bindings) == HOST_STATUS_OK) {
        runtime_bindings.unregister_runtime_bridge_bindings(
            static_cast<uint64_t>(session_handle)
        );
    }

    detach_bridge(env, static_cast<uint64_t>(session_handle));
}
