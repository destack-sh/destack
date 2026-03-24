#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_INTENT_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_INTENT_RUNTIME_H

#include "../types.h"

/// Query whether one outbound URL can be opened through one attached bridge session.
uint32_t call_can_open_url_for_session(
    uint64_t session_handle,
    NativeStringRef url,
    bool *is_supported
);
/// Open one outbound URL through one attached bridge session.
uint32_t call_open_url_for_session(uint64_t session_handle, NativeStringRef url);
/// Open one outbound path through one attached bridge session.
uint32_t call_open_path_for_session(uint64_t session_handle, NativeStringRef path);
/// Share one outbound text payload through one attached bridge session.
uint32_t call_share_text_for_session(
    uint64_t session_handle,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Share one outbound path list through one attached bridge session.
uint32_t call_share_paths_for_session(
    uint64_t session_handle,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent open-url event into the runtime ingress path.
RuntimeStatus send_intent_open_url(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef url
);
/// Send one intent open-file event into the runtime ingress path.
RuntimeStatus send_intent_open_file(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef path,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent share-text event into the runtime ingress path.
RuntimeStatus send_intent_share_text(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent share-files event into the runtime ingress path.
RuntimeStatus send_intent_share_files(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
);
/// Send one intent custom-action event into the runtime ingress path.
RuntimeStatus send_intent_custom_action(
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
