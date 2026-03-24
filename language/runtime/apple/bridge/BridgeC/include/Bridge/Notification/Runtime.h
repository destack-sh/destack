#ifndef RUNTIME_HOST_APPLE_BRIDGE_NOTIFICATION_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_NOTIFICATION_RUNTIME_H

#include "../Types.h"

/// Send one notification event into the runtime ingress path.
DestackRustRuntimeStatus send_notification_event(
    uint64_t session_handle,
    DestackRustNotificationEvent event
);

#endif
