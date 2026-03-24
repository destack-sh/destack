#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_PERMISSION_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_PERMISSION_RUNTIME_H

#include "../types.h"

/// Open one permission-settings surface through one attached bridge session.
uint32_t open_permission_settings_for_session(uint64_t session_handle);
/// Submit one permission request through one attached bridge session.
uint32_t call_permission_request_for_session(
    uint64_t session_handle,
    HostPermissionRequest request
);
/// Send one permission result into the runtime ingress path.
RuntimeStatus send_permission_result(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    NativeStringRef permission,
    bool granted
);

#endif
