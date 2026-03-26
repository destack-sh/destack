#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TEXT_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TEXT_RUNTIME_H

#include "../types.h"

/// Open one Android text-input session through one attached bridge session.
uint32_t call_text_open_for_session(
    uint64_t session_handle,
    HostTextOpenRequest request
);

/// Close one Android text-input session through one attached bridge session.
uint32_t call_text_close_for_session(
    uint64_t session_handle,
    uint64_t session_id
);

/// Update one Android text-input geometry through one attached bridge session.
uint32_t call_text_set_geometry_for_session(
    uint64_t session_handle,
    HostTextGeometryRequest request
);

/// Update one Android text-input state through one attached bridge session.
uint32_t call_text_set_state_for_session(
    uint64_t session_handle,
    HostTextStateRequest request
);

/// Send one text-input state event into the runtime ingress path.
RuntimeStatus send_text_input_state(
    uint64_t session_handle,
    uint64_t session_id,
    HostTextSessionState state
);

#endif
