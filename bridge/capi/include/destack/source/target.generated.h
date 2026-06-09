/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_TARGET_GENERATED_H
#define DESTACK_SOURCE_TARGET_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/package.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackTargetId {
    DestackPackageId package;
    char *key;
} DestackTargetId;

typedef struct DestackTargetIdArray {
    DestackTargetId *ptr;
    size_t len;
} DestackTargetIdArray;

typedef struct DestackOptionalTargetId {
    bool is_some;
    DestackTargetId value;
} DestackOptionalTargetId;

void destack_target_id_destroy(DestackTargetId *value);
void destack_target_id_array_destroy(DestackTargetIdArray array);

#ifdef __cplusplus
}
#endif

#endif
