#include "Bridge/Types.h"
#include "Bridge/Permission/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one permission result into the runtime ingress path.
DestackRustRuntimeStatus send_permission_result(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    NativeStringRef permission,
    bool granted
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_permission_result(
        session_handle,
        has_request_id,
        request_id,
        permission,
        granted
    );
}
