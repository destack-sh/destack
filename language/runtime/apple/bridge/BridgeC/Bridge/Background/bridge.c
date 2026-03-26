#include "Bridge/Types.h"
#include "Bridge/Background/Runtime.h"

/// Deliver one background event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_background_event(
    uint64_t session_handle,
    DestackRustBackgroundEvent event
) {
    return send_background_event(session_handle, event);
}
