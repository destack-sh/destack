#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_NOTIFICATION_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_NOTIFICATION_RUNTIME_H

#include "../types.h"

/// Post one notification through one attached bridge session.
uint32_t call_notification_post_for_session(
    uint64_t session_handle,
    HostNotificationRequest request
);
/// Cancel one notification through one attached bridge session.
uint32_t call_notification_cancel_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
);
/// Cancel every notification through one attached bridge session.
uint32_t call_notification_cancel_all_for_session(uint64_t session_handle);
/// Send one notification event into the runtime ingress path.
RuntimeStatus send_notification_event(
    uint64_t session_handle,
    HostNotificationEvent event
);

#endif
