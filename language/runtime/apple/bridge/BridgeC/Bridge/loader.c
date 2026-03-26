#include "Bridge/Types.h"
#include "Bridge/Loader.h"

#include <dlfcn.h>
#include <stddef.h>
#include <stdlib.h>

#define RESOLVE_SYMBOL_ONCE(slot, type, handle, name) \
    do { \
        if ((slot) == NULL) { \
            (slot) = (type)dlsym((handle), (name)); \
        } \
    } while (0)

static void *symbol_handle = NULL;

static RuntimeBindings runtime_bindings = {
    .register_runtime_bridge_bindings = NULL,
    .unregister_runtime_bridge_bindings = NULL,
    .notify_document_result = NULL,
    .notify_notification_event = NULL,
    .notify_intent_open_url = NULL,
    .notify_intent_open_file = NULL,
    .notify_intent_share_text = NULL,
    .notify_intent_share_files = NULL,
    .notify_intent_custom_action = NULL,
    .notify_location_sample = NULL,
    .notify_permission_result = NULL,
    .notify_text_input_state = NULL,
    .notify_background_event = NULL,
};

/// Resolve the shared runtime symbol handle for this process.
static void *resolve_symbol_handle(void) {
    if (symbol_handle != NULL) {
        return symbol_handle;
    }

    const char *library_path = getenv("DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY");
    if (library_path != NULL && library_path[0] != '\0') {
        symbol_handle = dlopen(library_path, RTLD_NOW | RTLD_GLOBAL);
        if (symbol_handle != NULL) {
            return symbol_handle;
        }
    }

    symbol_handle = dlopen(NULL, RTLD_NOW | RTLD_LOCAL);

    return symbol_handle;
}

/// Resolve the runtime bridge bindings from the current process.
uint32_t resolve_runtime_bindings(RuntimeBindings *out_bindings) {
    void *handle = resolve_symbol_handle();
    if (handle == NULL) {
        return 6;
    }

    RESOLVE_SYMBOL_ONCE(runtime_bindings.register_runtime_bridge_bindings, RegisterRuntimeBridgeBindingsFunction, handle, "destack_host_ios_register_runtime_bridge_bindings");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.unregister_runtime_bridge_bindings, UnregisterRuntimeBridgeBindingsFunction, handle, "destack_host_ios_unregister_runtime_bridge_bindings");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_document_result, NotifyDocumentResultFunction, handle, "destack_host_ios_notify_document_result");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_notification_event, NotifyNotificationEventFunction, handle, "destack_host_ios_notify_notification_event");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_intent_open_url, NotifyIntentOpenUrlFunction, handle, "destack_host_ios_notify_intent_open_url");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_intent_open_file, NotifyIntentOpenFileFunction, handle, "destack_host_ios_notify_intent_open_file");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_intent_share_text, NotifyIntentShareTextFunction, handle, "destack_host_ios_notify_intent_share_text");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_intent_share_files, NotifyIntentShareFilesFunction, handle, "destack_host_ios_notify_intent_share_files");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_intent_custom_action, NotifyIntentCustomActionFunction, handle, "destack_host_ios_notify_intent_custom_action");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_location_sample, NotifyLocationSampleFunction, handle, "destack_host_ios_notify_location_sample");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_permission_result, NotifyPermissionResultFunction, handle, "destack_host_ios_notify_permission_result");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_text_input_state, NotifyTextInputStateFunction, handle, "destack_host_ios_notify_text_input_state");
    RESOLVE_SYMBOL_ONCE(runtime_bindings.notify_background_event, NotifyBackgroundEventFunction, handle, "destack_host_ios_notify_background_event");

    if (
        runtime_bindings.register_runtime_bridge_bindings == NULL ||
        runtime_bindings.unregister_runtime_bridge_bindings == NULL ||
        runtime_bindings.notify_document_result == NULL ||
        runtime_bindings.notify_notification_event == NULL ||
        runtime_bindings.notify_intent_open_url == NULL ||
        runtime_bindings.notify_intent_open_file == NULL ||
        runtime_bindings.notify_intent_share_text == NULL ||
        runtime_bindings.notify_intent_share_files == NULL ||
        runtime_bindings.notify_intent_custom_action == NULL ||
        runtime_bindings.notify_location_sample == NULL ||
        runtime_bindings.notify_permission_result == NULL ||
        runtime_bindings.notify_text_input_state == NULL ||
        runtime_bindings.notify_background_event == NULL
    ) {
        return 3;
    }

    *out_bindings = runtime_bindings;

    return 0;
}

/// Register one mobile bridge callback table for one runtime session.
uint32_t destack_runtime_host_ios_register_runtime_bridge_bindings(
    uint64_t session_handle,
    IosRuntimeBridgeBindings callbacks
) {
    RuntimeBindings bindings = {0};
    uint32_t status = resolve_runtime_bindings(&bindings);
    if (status != 0) {
        return status;
    }

    return bindings.register_runtime_bridge_bindings(session_handle, callbacks);
}

/// Unregister one mobile bridge callback table for one runtime session.
void destack_runtime_host_ios_unregister_runtime_bridge_bindings(
    uint64_t session_handle
) {
    RuntimeBindings bindings = {0};
    if (resolve_runtime_bindings(&bindings) != 0) {
        return;
    }

    bindings.unregister_runtime_bridge_bindings(session_handle);
}
