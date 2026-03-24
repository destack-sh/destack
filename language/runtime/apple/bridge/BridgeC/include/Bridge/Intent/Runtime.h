#ifndef RUNTIME_HOST_APPLE_BRIDGE_INTENT_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_INTENT_RUNTIME_H

#include "../Types.h"

/// Send one intent open-url event into the runtime ingress path.
DestackRustRuntimeStatus send_intent_open_url(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef url
);
/// Send one intent open-file event into the runtime ingress path.
DestackRustRuntimeStatus send_intent_open_file(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef path,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent share-text event into the runtime ingress path.
DestackRustRuntimeStatus send_intent_share_text(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent share-files event into the runtime ingress path.
DestackRustRuntimeStatus send_intent_share_files(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent custom-action event into the runtime ingress path.
DestackRustRuntimeStatus send_intent_custom_action(
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

#endif
