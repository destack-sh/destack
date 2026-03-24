#include "Bridge/Types.h"
#include "Bridge/Intent/Runtime.h"

/// Convert one public string reference into one internal string reference.
static NativeStringRef native_string_ref(DestackRustStringRef value) {
    NativeStringRef native = {
        .data = value.data,
        .len = value.len,
    };

    return native;
}

/// Convert one public string slice into one internal string slice.
static NativeStringSlice native_string_slice(DestackRustStringSlice values) {
    NativeStringSlice native = {
        .data = (const NativeStringRef *)values.data,
        .len = values.len,
    };

    return native;
}

/// Deliver one intent open-url event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_url(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef url
) {
    return send_intent_open_url(
        session_handle,
        has_source,
        native_string_ref(source),
        native_string_ref(url)
    );
}

/// Deliver one intent open-file event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_file(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef path,
    bool has_content_type,
    DestackRustStringRef content_type
) {
    return send_intent_open_file(
        session_handle,
        has_source,
        native_string_ref(source),
        native_string_ref(path),
        has_content_type,
        native_string_ref(content_type)
    );
}

/// Deliver one intent share-text event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_text(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef text,
    bool has_content_type,
    DestackRustStringRef content_type
) {
    return send_intent_share_text(
        session_handle,
        has_source,
        native_string_ref(source),
        native_string_ref(text),
        has_content_type,
        native_string_ref(content_type)
    );
}

/// Deliver one intent share-files event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_files(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringSlice paths,
    bool has_content_type,
    DestackRustStringRef content_type
) {
    return send_intent_share_files(
        session_handle,
        has_source,
        native_string_ref(source),
        native_string_slice(paths),
        has_content_type,
        native_string_ref(content_type)
    );
}

/// Deliver one intent custom-action event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_custom_action(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef action,
    bool has_url,
    DestackRustStringRef url,
    DestackRustStringSlice paths,
    bool has_text,
    DestackRustStringRef text,
    bool has_content_type,
    DestackRustStringRef content_type
) {
    return send_intent_custom_action(
        session_handle,
        has_source,
        native_string_ref(source),
        native_string_ref(action),
        has_url,
        native_string_ref(url),
        native_string_slice(paths),
        has_text,
        native_string_ref(text),
        has_content_type,
        native_string_ref(content_type)
    );
}
