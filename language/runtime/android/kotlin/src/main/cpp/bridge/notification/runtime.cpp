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

/// Send one notification event into the runtime ingress path.
RuntimeStatus send_notification_event(
    uint64_t session_handle,
    HostNotificationEvent event
) {
    RuntimeBindings bindings = {};
    RuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_notification_event(session_handle, event);
}
