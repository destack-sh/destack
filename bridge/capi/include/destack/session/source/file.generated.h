/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_SOURCE_FILE_GENERATED_H
#define DESTACK_SESSION_SOURCE_FILE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/module.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackChange {
    char *path;
    char *uri;
    bool is_removed;
    DestackOptionalModuleId module_id;
} DestackChange;

typedef struct DestackChangeArray {
    DestackChange *ptr;
    size_t len;
} DestackChangeArray;

typedef struct DestackOptionalChange {
    bool is_some;
    DestackChange value;
} DestackOptionalChange;

void destack_change_destroy(DestackChange *value);
void destack_change_array_destroy(DestackChangeArray array);

#ifdef __cplusplus
}
#endif

#endif
