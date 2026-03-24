#include "loader.h"

#include <dlfcn.h>
#include <stdlib.h>

static void *symbol_handle = NULL;

static void *resolve_testing_symbol_handle(void) {
    if (symbol_handle != NULL) {
        return symbol_handle;
    }

    const char *library_path = getenv("DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY");
    if (library_path != NULL && library_path[0] != '\0') {
        symbol_handle = dlopen(library_path, RTLD_NOW | RTLD_GLOBAL);
        if (symbol_handle != NULL) {
            return symbol_handle;
        }
    }

    symbol_handle = dlopen(NULL, RTLD_NOW | RTLD_LOCAL);

    return symbol_handle;
}

/// Resolve one test symbol from the loaded runtime-host bridge test library.
void *resolve_testing_symbol(const char *name) {
    void *handle = resolve_testing_symbol_handle();
    if (handle == NULL) {
        return NULL;
    }

    return dlsym(handle, name);
}
