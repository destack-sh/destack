#include "RuntimeHostAppleBridgeTest.h"
#include "loader.h"

#include <Bridge/Types.h>

typedef uint32_t (*SubmitTestDocumentRequestFunction)(
    uint64_t session_handle,
    DestackRustDocumentRequest request
);

static SubmitTestDocumentRequestFunction submit_test_document_request = NULL;

static int resolve_document_testing_symbols(void) {
    if (submit_test_document_request == NULL) {
        submit_test_document_request = (SubmitTestDocumentRequestFunction)resolve_testing_symbol(
            "destack_host_test_ios_submit_document_request"
        );
    }

    return submit_test_document_request != NULL;
}

/// Submit one iOS document request through the live runtime host bridge.
uint32_t destack_runtime_host_ios_test_submit_document_request(
    uint64_t session_handle,
    DestackRustDocumentRequest request
) {
    if (!resolve_document_testing_symbols()) {
        return host_status_not_found;
    }

    return submit_test_document_request(session_handle, request);
}
