#include "Bridge/Types.h"
#include "Bridge/Location/Runtime.h"

/// Deliver one location sample into the runtime ingress path.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_location_sample(
    uint64_t session_handle,
    DestackRustStringRef watch_id,
    DestackRustLocationSample sample
) {
    NativeStringRef watch_id_ref = {
        .data = watch_id.data,
        .len = watch_id.len,
    };

    return send_location_sample(
        session_handle,
        watch_id_ref,
        sample
    );
}
