#ifndef RUNTIME_HOST_APPLE_BRIDGE_LOADER_H
#define RUNTIME_HOST_APPLE_BRIDGE_LOADER_H

#include "Types.h"

/// The resolved runtime bridge bindings for this process.
typedef struct RuntimeBindings {
    /// The runtime function that registers one mobile bridge callback table for one session.
    RegisterRuntimeBridgeBindingsFunction register_runtime_bridge_bindings;
    /// The runtime function that unregisters one mobile bridge callback table for one session.
    UnregisterRuntimeBridgeBindingsFunction unregister_runtime_bridge_bindings;
    /// The runtime function that receives one document result.
    NotifyDocumentResultFunction notify_document_result;
    /// The runtime function that receives one notification event.
    NotifyNotificationEventFunction notify_notification_event;
    /// The runtime function that receives one intent open-url event.
    NotifyIntentOpenUrlFunction notify_intent_open_url;
    /// The runtime function that receives one intent open-file event.
    NotifyIntentOpenFileFunction notify_intent_open_file;
    /// The runtime function that receives one intent share-text event.
    NotifyIntentShareTextFunction notify_intent_share_text;
    /// The runtime function that receives one intent share-files event.
    NotifyIntentShareFilesFunction notify_intent_share_files;
    /// The runtime function that receives one intent custom-action event.
    NotifyIntentCustomActionFunction notify_intent_custom_action;
    /// The runtime function that receives one location sample.
    NotifyLocationSampleFunction notify_location_sample;
    /// The runtime function that receives one permission result.
    NotifyPermissionResultFunction notify_permission_result;
    /// The runtime function that receives one text-input state event.
    NotifyTextInputStateFunction notify_text_input_state;
    /// The runtime function that receives one background event.
    NotifyBackgroundEventFunction notify_background_event;
} RuntimeBindings;

/// Resolve the runtime bridge bindings from the current process.
uint32_t resolve_runtime_bindings(RuntimeBindings *out_bindings);

#endif
