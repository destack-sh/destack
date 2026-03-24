#ifndef RUNTIME_HOST_APPLE_BRIDGE_PERMISSION_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_PERMISSION_RUNTIME_H

#include "../Types.h"

/// Send one permission result into the runtime ingress path.
DestackRustRuntimeStatus send_permission_result(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    NativeStringRef permission,
    bool granted
);

#endif
