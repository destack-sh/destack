/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_MODULE_GENERATED_H
#define DESTACK_SESSION_MODULE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/module.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackModule {
    DestackModuleId id;
} DestackModule;

typedef struct DestackModuleArray {
    DestackModule *ptr;
    size_t len;
} DestackModuleArray;

typedef struct DestackOptionalModule {
    bool is_some;
    DestackModule value;
} DestackOptionalModule;

void destack_module_destroy(DestackModule *value);
void destack_module_array_destroy(DestackModuleArray array);

#ifdef __cplusplus
}
#endif

#endif
