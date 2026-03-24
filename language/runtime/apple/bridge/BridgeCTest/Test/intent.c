#include "RuntimeHostAppleBridgeTest.h"
#include "loader.h"

#include <Bridge/Types.h>

typedef uint32_t (*OpenTestIntentCanOpenUrlFunction)(
    uint64_t session_handle,
    NativeStringRef url,
    bool *is_supported
);

static OpenTestIntentCanOpenUrlFunction open_test_intent_can_open_url = NULL;

static int resolve_intent_testing_symbols(void) {
    if (open_test_intent_can_open_url == NULL) {
        open_test_intent_can_open_url = (OpenTestIntentCanOpenUrlFunction)resolve_testing_symbol(
            "destack_host_test_ios_intent_can_open_url"
        );
    }

    return open_test_intent_can_open_url != NULL;
}

/// Submit one iOS can-open-url request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_intent_can_open_url(
    uint64_t session_handle,
    DestackRustStringRef url,
    bool *is_supported
) {
    NativeStringRef native_url = {
        .data = url.data,
        .len = url.len,
    };

    if (!resolve_intent_testing_symbols()) {
        return host_status_not_found;
    }

    return open_test_intent_can_open_url(session_handle, native_url, is_supported);
}
