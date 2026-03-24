#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TESTING_LOADER_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TESTING_LOADER_H

#include "../types.h"

/// Resolve one testing symbol from the loaded runtime-host testing library.
void *resolve_testing_symbol(const char *name);

#endif
