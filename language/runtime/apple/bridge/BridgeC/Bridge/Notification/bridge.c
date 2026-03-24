#include "Bridge/Types.h"
#include "Bridge/Notification/Runtime.h"

/// Deliver one notification event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_notification_event(
    uint64_t session_handle,
    DestackRustNotificationEvent event
) {
    return send_notification_event(session_handle, event);
}
