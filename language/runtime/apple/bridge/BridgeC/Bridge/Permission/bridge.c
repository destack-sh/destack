#include "Bridge/Types.h"
#include "Bridge/Permission/Runtime.h"

/// Deliver one permission result into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_permission_result(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    const uint8_t *permission,
    uint32_t permission_len,
    bool granted
) {
    NativeStringRef permission_ref = {
        .data = permission,
        .len = permission_len,
    };

    return send_permission_result(
        session_handle,
        has_request_id,
        request_id,
        permission_ref,
        granted
    );
}
