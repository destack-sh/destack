#include "Bridge/Types.h"
#include "Bridge/Text/Runtime.h"

/// Deliver one text-input state event into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_text_input_state(
    uint64_t session_handle,
    uint64_t session_id,
    DestackRustTextSessionState state
) {
    return send_text_input_state(session_handle, session_id, state);
}
