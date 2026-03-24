#ifndef RUNTIME_HOST_APPLE_BRIDGE_LOCATION_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_LOCATION_RUNTIME_H

#include "../Types.h"

/// Send one location sample into the runtime ingress path.
DestackRustRuntimeStatus send_location_sample(
    uint64_t session_handle,
    NativeStringRef watch_id,
    DestackRustLocationSample sample
);

#endif
