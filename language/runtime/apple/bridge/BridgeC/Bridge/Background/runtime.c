#include "Bridge/Types.h"
#include "Bridge/Background/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one background event into the runtime ingress path.
DestackRustRuntimeStatus send_background_event(
    uint64_t session_handle,
    DestackRustBackgroundEvent event
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_background_event(session_handle, event);
}
