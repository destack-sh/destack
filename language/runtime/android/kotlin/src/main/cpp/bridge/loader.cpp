#include "types.h"
#include "loader.h"

#include <dlfcn.h>
#include <stdlib.h>

namespace {

void *symbol_handle = nullptr;
RuntimeBindings runtime_bindings = {
    .register_runtime_bridge_bindings = nullptr,
    .unregister_runtime_bridge_bindings = nullptr,
    .notify_document_result = nullptr,
    .notify_notification_event = nullptr,
    .notify_intent_open_url = nullptr,
    .notify_intent_open_file = nullptr,
    .notify_intent_share_text = nullptr,
    .notify_intent_share_files = nullptr,
    .notify_intent_custom_action = nullptr,
    .notify_permission_result = nullptr,
    .notify_location_sample = nullptr,
};

template <typename SymbolFunction>
void resolve_symbol_once(SymbolFunction *slot, void *handle, const char *name) {
    if (*slot != nullptr) {
        return;
    }

    *slot = reinterpret_cast<SymbolFunction>(dlsym(handle, name));
}

/// Resolve the shared runtime symbol handle for this process.
void *resolve_symbol_handle() {
    if (symbol_handle != nullptr) {
        return symbol_handle;
    }

    const char *library_path = getenv("DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY");
    if (library_path != nullptr && library_path[0] != '\0') {
        symbol_handle = dlopen(library_path, RTLD_NOW | RTLD_GLOBAL);
        if (symbol_handle != nullptr) {
            return symbol_handle;
        }
    }

    symbol_handle = dlopen(nullptr, RTLD_NOW | RTLD_LOCAL);

    return symbol_handle;
}

}

/// Resolve the runtime bridge bindings from the current process.
uint32_t resolve_runtime_bindings(RuntimeBindings *out_bindings) {
    void *handle = resolve_symbol_handle();
    if (handle == nullptr) {
        return HOST_STATUS_FAILED;
    }

    resolve_symbol_once(&runtime_bindings.register_runtime_bridge_bindings, handle, "destack_host_android_register_runtime_bridge_bindings");
    resolve_symbol_once(&runtime_bindings.unregister_runtime_bridge_bindings, handle, "destack_host_android_unregister_runtime_bridge_bindings");
    resolve_symbol_once(&runtime_bindings.notify_document_result, handle, "destack_host_android_notify_document_result");
    resolve_symbol_once(&runtime_bindings.notify_notification_event, handle, "destack_host_android_notify_notification_event");
    resolve_symbol_once(&runtime_bindings.notify_intent_open_url, handle, "destack_host_android_notify_intent_open_url");
    resolve_symbol_once(&runtime_bindings.notify_intent_open_file, handle, "destack_host_android_notify_intent_open_file");
    resolve_symbol_once(&runtime_bindings.notify_intent_share_text, handle, "destack_host_android_notify_intent_share_text");
    resolve_symbol_once(&runtime_bindings.notify_intent_share_files, handle, "destack_host_android_notify_intent_share_files");
    resolve_symbol_once(&runtime_bindings.notify_intent_custom_action, handle, "destack_host_android_notify_intent_custom_action");
    resolve_symbol_once(&runtime_bindings.notify_permission_result, handle, "destack_host_android_notify_permission_result");
    resolve_symbol_once(&runtime_bindings.notify_location_sample, handle, "destack_host_android_notify_location_sample");

    if (
        runtime_bindings.register_runtime_bridge_bindings == nullptr ||
        runtime_bindings.unregister_runtime_bridge_bindings == nullptr ||
        runtime_bindings.notify_document_result == nullptr ||
        runtime_bindings.notify_notification_event == nullptr ||
        runtime_bindings.notify_intent_open_url == nullptr ||
        runtime_bindings.notify_intent_open_file == nullptr ||
        runtime_bindings.notify_intent_share_text == nullptr ||
        runtime_bindings.notify_intent_share_files == nullptr ||
        runtime_bindings.notify_intent_custom_action == nullptr ||
        runtime_bindings.notify_permission_result == nullptr ||
        runtime_bindings.notify_location_sample == nullptr
    ) {
        return HOST_STATUS_NOT_FOUND;
    }

    *out_bindings = runtime_bindings;

    return 0;
}
