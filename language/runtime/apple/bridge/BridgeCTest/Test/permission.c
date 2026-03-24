#include "RuntimeHostAppleBridgeTest.h"
#include "loader.h"

#include <Bridge/Types.h>

typedef uint32_t (*SubmitTestPermissionRequestFunction)(
    uint64_t session_handle,
    DestackRustPermissionRequest request
);
typedef uint32_t (*OpenTestPermissionSettingsFunction)(
    uint64_t session_handle
);

static SubmitTestPermissionRequestFunction submit_test_permission_request = NULL;
static OpenTestPermissionSettingsFunction open_test_permission_settings = NULL;

static int resolve_permission_testing_symbols(void) {
    if (submit_test_permission_request == NULL) {
        submit_test_permission_request = (SubmitTestPermissionRequestFunction)resolve_testing_symbol(
            "destack_host_test_ios_submit_permission_request"
        );
    }

    if (open_test_permission_settings == NULL) {
        open_test_permission_settings = (OpenTestPermissionSettingsFunction)resolve_testing_symbol(
            "destack_host_test_ios_open_permission_settings"
        );
    }

    return
        submit_test_permission_request != NULL &&
        open_test_permission_settings != NULL;
}

/// Submit one iOS permission request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_permission_request(
    uint64_t session_handle,
    DestackRustPermissionRequest request
) {
    if (!resolve_permission_testing_symbols()) {
        return host_status_not_found;
    }

    return submit_test_permission_request(session_handle, request);
}

/// Submit one iOS permission-settings request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_open_permission_settings(
    uint64_t session_handle
) {
    if (!resolve_permission_testing_symbols()) {
        return host_status_not_found;
    }

    return open_test_permission_settings(session_handle);
}
