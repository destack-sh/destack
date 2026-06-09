/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_MODULE_GENERATED_H
#define DESTACK_SOURCE_MODULE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/package.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackModuleId {
    DestackPackageId package;
    char *key;
} DestackModuleId;

typedef struct DestackModuleIdArray {
    DestackModuleId *ptr;
    size_t len;
} DestackModuleIdArray;

typedef struct DestackOptionalModuleId {
    bool is_some;
    DestackModuleId value;
} DestackOptionalModuleId;

void destack_module_id_destroy(DestackModuleId *value);
void destack_module_id_array_destroy(DestackModuleIdArray array);

#ifdef __cplusplus
}
#endif

#endif
