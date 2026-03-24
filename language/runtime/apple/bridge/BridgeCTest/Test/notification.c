#include "RuntimeHostAppleBridgeTest.h"
#include "loader.h"

#include <Bridge/Types.h>

typedef uint32_t (*SubmitTestNotificationPostFunction)(
    uint64_t session_handle,
    DestackRustNotificationRequest request
);

static SubmitTestNotificationPostFunction submit_test_notification_post = NULL;

static int resolve_notification_testing_symbols(void) {
    if (submit_test_notification_post == NULL) {
        submit_test_notification_post = (SubmitTestNotificationPostFunction)resolve_testing_symbol(
            "destack_host_ios_notification_post"
        );
    }

    return submit_test_notification_post != NULL;
}

/// Submit one iOS notification request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_notification_post(
    uint64_t session_handle,
    DestackRustNotificationRequest request
) {
    if (!resolve_notification_testing_symbols()) {
        return host_status_not_found;
    }

    return submit_test_notification_post(session_handle, request);
}
