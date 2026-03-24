#ifndef RUNTIME_HOST_APPLE_BRIDGE_TESTING_H
#define RUNTIME_HOST_APPLE_BRIDGE_TESTING_H

#include <stdbool.h>
#include <stdint.h>

#include <RuntimeHostAppleBridge.h>

/// Open one live iOS host session for one bridge integration test.
uint64_t destack_runtime_host_ios_test_open_session(void);

/// Close one live iOS host session for one bridge integration test.
void destack_runtime_host_ios_test_close_session(
    uint64_t session_handle
);

/// Submit one iOS document request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_document_request(
    uint64_t session_handle,
    DestackRustDocumentRequest request
);

/// Submit one iOS permission request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_permission_request(
    uint64_t session_handle,
    DestackRustPermissionRequest request
);

/// Submit one iOS permission-settings request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_open_permission_settings(
    uint64_t session_handle
);

/// Submit one iOS can-open-url request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_intent_can_open_url(
    uint64_t session_handle,
    DestackRustStringRef url,
    bool *is_supported
);

/// Submit one iOS notification request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_notification_post(
    uint64_t session_handle,
    DestackRustNotificationRequest request
);

#endif
