#include "../types.h"
#include "../loader.h"
#include "runtime.h"

namespace {

/// Convert one host status code into one runtime status.
RuntimeStatus runtime_status_from_code(uint32_t code) {
    RuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

}

/// Send one intent open-url event into the runtime ingress path.
RuntimeStatus send_intent_open_url(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef url
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_intent_open_url(session_handle, has_source, source, url);
}

/// Send one intent open-file event into the runtime ingress path.
RuntimeStatus send_intent_open_file(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef path,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_intent_open_file(
        session_handle,
        has_source,
        source,
        path,
        has_mime_type,
        mime_type
    );
}

/// Send one intent share-text event into the runtime ingress path.
RuntimeStatus send_intent_share_text(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringRef text,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_intent_share_text(
        session_handle,
        has_source,
        source,
        text,
        has_mime_type,
        mime_type
    );
}

/// Send one intent share-files event into the runtime ingress path.
RuntimeStatus send_intent_share_files(
    uint64_t session_handle,
    bool has_source,
    NativeStringRef source,
    NativeStringSlice paths,
    bool has_mime_type,
    NativeStringRef mime_type
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_intent_share_files(
        session_handle,
        has_source,
        source,
        paths,
        has_mime_type,
        mime_type
    );
}

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
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_intent_custom_action(
        session_handle,
        has_source,
        source,
        action,
        has_url,
        url,
        paths,
        has_text,
        text,
        has_mime_type,
        mime_type
    );
}
