#ifndef RUNTIME_HOST_APPLE_BRIDGE_REGISTRY_H
#define RUNTIME_HOST_APPLE_BRIDGE_REGISTRY_H

#include "Types.h"

/// Resolve the document callback for one runtime session.
DestackRustDocumentCallback resolve_document_callback(uint64_t session_handle);
/// Store the document callback for one runtime session.
bool store_document_callback(
    uint64_t session_handle,
    DestackRustDocumentCallback callback
);
/// Clear the document callback for one runtime session.
void clear_document_callback(uint64_t session_handle);

/// The permission callbacks stored for one runtime session.
typedef struct PermissionCallbacks {
    /// The open-settings callback.
    DestackRustPermissionOpenSettingsCallback open_settings;
    /// The permission request callback.
    DestackRustPermissionRequestCallback request;
} PermissionCallbacks;

/// The intent callbacks stored for one runtime session.
typedef struct IntentCallbacks {
    /// The can-open-url callback.
    DestackRustIntentCanOpenUrlCallback can_open_url;
    /// The open-url callback.
    DestackRustIntentOpenUrlCallback open_url;
    /// The open-path callback.
    DestackRustIntentOpenPathCallback open_path;
    /// The share-text callback.
    DestackRustIntentShareTextCallback share_text;
    /// The share-paths callback.
    DestackRustIntentSharePathsCallback share_paths;
} IntentCallbacks;

/// The notification callbacks stored for one runtime session.
typedef struct NotificationCallbacks {
    /// The post callback.
    uint32_t (*post)(uint64_t session_handle, DestackRustNotificationRequest request);
    /// The cancel callback.
    uint32_t (*cancel)(uint64_t session_handle, DestackRustStringRef identifier);
    /// The cancel-all callback.
    uint32_t (*cancel_all)(uint64_t session_handle);
} NotificationCallbacks;

/// Resolve the permission callbacks for one runtime session.
PermissionCallbacks resolve_permission_callbacks(uint64_t session_handle);
/// Store the permission callbacks for one runtime session.
bool store_permission_callbacks(
    uint64_t session_handle,
    DestackRustPermissionOpenSettingsCallback open_settings,
    DestackRustPermissionRequestCallback request
);
/// Clear the permission callbacks for one runtime session.
void clear_permission_callbacks(uint64_t session_handle);

/// Resolve the intent callbacks for one runtime session.
IntentCallbacks resolve_intent_callbacks(uint64_t session_handle);
/// Store the intent callbacks for one runtime session.
bool store_intent_callbacks(
    uint64_t session_handle,
    DestackRustIntentCanOpenUrlCallback can_open_url,
    DestackRustIntentOpenUrlCallback open_url,
    DestackRustIntentOpenPathCallback open_path,
    DestackRustIntentShareTextCallback share_text,
    DestackRustIntentSharePathsCallback share_paths
);
/// Clear the intent callbacks for one runtime session.
void clear_intent_callbacks(uint64_t session_handle);

/// Resolve the notification callbacks for one runtime session.
NotificationCallbacks resolve_notification_callbacks(uint64_t session_handle);
/// Store the notification callbacks for one runtime session.
bool store_notification_callbacks(
    uint64_t session_handle,
    uint32_t (*post)(uint64_t session_handle, DestackRustNotificationRequest request),
    uint32_t (*cancel)(uint64_t session_handle, DestackRustStringRef identifier),
    uint32_t (*cancel_all)(uint64_t session_handle)
);
/// Clear the notification callbacks for one runtime session.
void clear_notification_callbacks(uint64_t session_handle);

#endif
