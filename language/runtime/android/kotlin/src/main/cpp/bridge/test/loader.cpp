#include "../types.h"
#include "loader.h"

#include <dlfcn.h>
#include <stdlib.h>

namespace {

constexpr jint jni_version = JNI_VERSION_1_6;
void *symbol_handle = nullptr;

}

/// Resolve one handle to the loaded runtime-host testing library.
static void *resolve_testing_symbol_handle() {
    if (symbol_handle != nullptr) {
        return symbol_handle;
    }

    const char *library_path = getenv("DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY");
    if (library_path != nullptr && library_path[0] != '\0') {
        symbol_handle = dlopen(library_path, RTLD_NOW | RTLD_GLOBAL);
        if (symbol_handle != nullptr) {
            return symbol_handle;
        }
    }

    symbol_handle = dlopen(nullptr, RTLD_NOW | RTLD_LOCAL);

    return symbol_handle;
}

/// Resolve one testing symbol from the loaded runtime-host testing library.
void *resolve_testing_symbol(const char *name) {
    void *handle = resolve_testing_symbol_handle();
    if (handle == nullptr) {
        return nullptr;
    }

    return dlsym(handle, name);
}

/// Capture the Java VM for the Android bridge test library.
extern "C" JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM * /* vm */, void * /* reserved */) {
    return jni_version;
}
