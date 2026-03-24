#ifndef RUNTIME_HOST_APPLE_BRIDGE_TESTING_LOADER_H
#define RUNTIME_HOST_APPLE_BRIDGE_TESTING_LOADER_H

#include <stdint.h>

static const uint32_t host_status_ok = 0;
static const uint32_t host_status_not_found = 3;
static const uint32_t host_status_failed = 6;

/// Resolve one test symbol from the loaded runtime-host bridge test library.
void *resolve_testing_symbol(const char *name);

#endif
