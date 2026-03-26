#include "Bridge/Types.h"
#include "Bridge/Text/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one text-input state event into the runtime ingress path.
DestackRustRuntimeStatus send_text_input_state(
    uint64_t session_handle,
    uint64_t session_id,
    DestackRustTextSessionState state
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_text_input_state(session_handle, session_id, state);
}
