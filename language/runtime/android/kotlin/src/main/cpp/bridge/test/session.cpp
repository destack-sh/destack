#include "../types.h"
#include "loader.h"

namespace {

using OpenTestSessionFunction = uint64_t (*)();
using CloseTestSessionFunction = void (*)(uint64_t);

OpenTestSessionFunction open_test_session = nullptr;
CloseTestSessionFunction close_test_session = nullptr;

/// Resolve the session testing symbols.
bool resolve_session_symbols() {
    if (open_test_session == nullptr) {
        open_test_session = reinterpret_cast<OpenTestSessionFunction>(
            resolve_testing_symbol("destack_host_test_android_open_session")
        );
    }

    if (close_test_session == nullptr) {
        close_test_session = reinterpret_cast<CloseTestSessionFunction>(
            resolve_testing_symbol("destack_host_test_android_close_session")
        );
    }

    return open_test_session != nullptr && close_test_session != nullptr;
}

}

/// Open one live runtime session for one Android bridge test.
extern "C" JNIEXPORT jlong JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeOpenTestSession(
    JNIEnv * /* env */,
    jobject /* testing */
) {
    if (!resolve_session_symbols()) {
        return 0;
    }

    return static_cast<jlong>(open_test_session());
}

/// Close one live runtime session for one Android bridge test.
extern "C" JNIEXPORT void JNICALL
Java_dev_destack_runtime_android_bridge_RuntimeAbiTest_nativeCloseTestSession(
    JNIEnv * /* env */,
    jobject /* testing */,
    jlong session_handle
) {
    if (!resolve_session_symbols()) {
        return;
    }

    close_test_session(static_cast<uint64_t>(session_handle));
}
