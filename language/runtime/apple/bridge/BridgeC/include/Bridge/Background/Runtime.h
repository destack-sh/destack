#ifndef RUNTIME_HOST_APPLE_BRIDGE_BACKGROUND_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_BACKGROUND_RUNTIME_H

#include "../Types.h"

/// Send one background event into the runtime ingress path.
DestackRustRuntimeStatus send_background_event(
    uint64_t session_handle,
    DestackRustBackgroundEvent event
);

#endif
