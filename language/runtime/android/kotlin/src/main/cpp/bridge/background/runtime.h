#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_BACKGROUND_RUNTIME_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_BACKGROUND_RUNTIME_H

#include "../types.h"

/// Read Android background scheduler status through one attached bridge session.
uint32_t call_background_status_for_session(
    uint64_t session_handle,
    HostBackgroundStatus *output_status
);

/// List Android background tasks through one attached bridge session.
uint32_t call_background_list_for_session(
    uint64_t session_handle,
    NativeArray<HostBackgroundTaskDescriptor> *output_descriptors
);

/// Register one Android background task through one attached bridge session.
uint32_t call_background_register_for_session(
    uint64_t session_handle,
    HostBackgroundTaskOptions options
);

/// Unregister one Android background task through one attached bridge session.
uint32_t call_background_unregister_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
);

/// Trigger one Android background task through one attached bridge session.
uint32_t call_background_trigger_test_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    bool *is_triggered
);

/// Complete one Android background task through one attached bridge session.
uint32_t call_background_complete_for_session(
    uint64_t session_handle,
    NativeStringRef execution_id,
    HostBackgroundTaskResult result
);

/// Send one background event into the runtime ingress path.
RuntimeStatus send_background_event(
    uint64_t session_handle,
    HostBackgroundEvent event
);

#endif
