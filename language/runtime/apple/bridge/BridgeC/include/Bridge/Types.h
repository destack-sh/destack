#ifndef RUNTIME_HOST_APPLE_BRIDGE_TYPES_H
#define RUNTIME_HOST_APPLE_BRIDGE_TYPES_H

#include "../RuntimeHostAppleBridge.h"

#include <stdbool.h>
#include <stdint.h>

typedef struct NativeSlice {
    uint8_t *data;
    uint32_t len;
} NativeSlice;

typedef struct NativeStringRef {
    const uint8_t *data;
    uint32_t len;
} NativeStringRef;

typedef struct NativeStringSlice {
    const NativeStringRef *data;
    uint32_t len;
} NativeStringSlice;

typedef uint32_t (*RegisterRuntimeBridgeBindingsFunction)(
    uint64_t session_handle,
    IosRuntimeBridgeBindings callbacks
);
typedef void (*UnregisterRuntimeBridgeBindingsFunction)(
    uint64_t session_handle
);

typedef DestackRustRuntimeStatus (*NotifyDocumentResultFunction)(
    uint64_t session_handle,
    uint64_t request_id,
    DestackRustDocumentDescriptorSlice documents
);

typedef DestackRustRuntimeStatus (*NotifyNotificationEventFunction)(
    uint64_t session_handle,
    DestackRustNotificationEvent event
);

typedef DestackRustRuntimeStatus (*NotifyIntentOpenUrlFunction)(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef url
);

typedef DestackRustRuntimeStatus (*NotifyIntentOpenFileFunction)(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef path,
    bool has_mime_type,
    NativeStringRef mime_type
);

typedef DestackRustRuntimeStatus (*NotifyIntentShareTextFunction)(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);

typedef DestackRustRuntimeStatus (*NotifyIntentShareFilesFunction)(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
);

typedef DestackRustRuntimeStatus (*NotifyIntentCustomActionFunction)(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef action,
    bool has_url,
    NativeStringRef url,
    NativeStringSlice paths,
    bool has_text,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);

typedef DestackRustRuntimeStatus (*NotifyLocationSampleFunction)(
    uint64_t session_handle,
    NativeStringRef watch_id,
    DestackRustLocationSample sample
);

typedef DestackRustRuntimeStatus (*NotifyPermissionResultFunction)(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    NativeStringRef permission,
    bool granted
);

typedef DestackRustRuntimeStatus (*NotifyTextInputStateFunction)(
    uint64_t session_handle,
    uint64_t session_id,
    DestackRustTextSessionState state
);

typedef DestackRustRuntimeStatus (*NotifyBackgroundEventFunction)(
    uint64_t session_handle,
    DestackRustBackgroundEvent event
);

#endif
