#include "RuntimeHostAppleBridgeTest.h"
#include "loader.h"

typedef uint64_t (*OpenTestSessionFunction)(void);
typedef void (*CloseTestSessionFunction)(
    uint64_t session_handle
);

static OpenTestSessionFunction open_test_session = NULL;
static CloseTestSessionFunction close_test_session = NULL;

static int resolve_session_symbols(void) {
    if (open_test_session == NULL) {
        open_test_session = (OpenTestSessionFunction)resolve_testing_symbol(
            "destack_host_test_ios_open_session"
        );
    }

    if (close_test_session == NULL) {
        close_test_session = (CloseTestSessionFunction)resolve_testing_symbol(
            "destack_host_test_ios_close_session"
        );
    }

    return open_test_session != NULL && close_test_session != NULL;
}

/// Open one live iOS host session for one bridge integration test.
uint64_t destack_runtime_host_ios_test_open_session(void) {
    if (!resolve_session_symbols()) {
        return 0;
    }

    return open_test_session();
}

/// Close one live iOS host session for one bridge integration test.
void destack_runtime_host_ios_test_close_session(
    uint64_t session_handle
) {
    if (!resolve_session_symbols()) {
        return;
    }

    close_test_session(session_handle);
}
