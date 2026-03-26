#ifndef RUNTIME_HOST_APPLE_BRIDGE_TEXT_RUNTIME_H
#define RUNTIME_HOST_APPLE_BRIDGE_TEXT_RUNTIME_H

#include "../Types.h"

/// Send one text-input state event into the runtime ingress path.
DestackRustRuntimeStatus send_text_input_state(
    uint64_t session_handle,
    uint64_t session_id,
    DestackRustTextSessionState state
);

#endif
