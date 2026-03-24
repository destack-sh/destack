#include "Bridge/Types.h"
#include "Bridge/Location/Runtime.h"
#include "Bridge/Loader.h"

/// Convert one host status code into one runtime status.
static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {
    DestackRustRuntimeStatus status = {
        .code = code,
        .error_id = 0,
    };

    return status;
}

/// Send one location sample into the runtime ingress path.
DestackRustRuntimeStatus send_location_sample(
    uint64_t session_handle,
    NativeStringRef watch_id,
    DestackRustLocationSample sample
) {
    RuntimeBindings bindings = {0};
    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));
    if (status.code != 0) {
        return status;
    }

    return bindings.notify_location_sample(
        session_handle,
        watch_id,
        sample
    );
}
