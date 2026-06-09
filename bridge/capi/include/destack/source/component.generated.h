/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_COMPONENT_GENERATED_H
#define DESTACK_SOURCE_COMPONENT_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackComponentId {
    char *id;
} DestackComponentId;

typedef struct DestackComponentIdArray {
    DestackComponentId *ptr;
    size_t len;
} DestackComponentIdArray;

typedef struct DestackOptionalComponentId {
    bool is_some;
    DestackComponentId value;
} DestackOptionalComponentId;

void destack_component_id_destroy(DestackComponentId *value);
void destack_component_id_array_destroy(DestackComponentIdArray array);

#ifdef __cplusplus
}
#endif

#endif
