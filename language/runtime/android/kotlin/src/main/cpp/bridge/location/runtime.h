#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_LOCATION_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_LOCATION_RUNTIME_H

#include "../types.h"

/// Read whether Android location services are enabled through one attached bridge session.
uint32_t call_location_services_enabled_for_session(
    uint64_t session_handle,
    bool *is_enabled
);

/// Read one Android last-known location sample through one attached bridge session.
uint32_t call_location_last_known_for_session(
    uint64_t session_handle,
    LocationSample *sample
);

/// Open one Android location watch through one attached bridge session.
uint32_t call_location_watch_open_for_session(
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationWatchOptions options
);

/// Close one Android location watch through one attached bridge session.
uint32_t call_location_watch_close_for_session(
    uint64_t session_handle,
    NativeStringRef watch_id
);

/// Send one location sample into the runtime ingress path.
RuntimeStatus send_location_sample(
    uint64_t session_handle,
    NativeStringRef watch_id,
    LocationSample sample
);

#endif
