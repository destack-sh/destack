#include "Bridge/Types.h"
#include "Bridge/Notification/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one notification event into the runtime ingress path.
DestackRustRuntimeStatus send_notification_event(
    uint64_t session_handle,
    DestackRustNotificationEvent event
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_notification_event(session_handle, event);
}
