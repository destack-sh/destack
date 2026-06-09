/* generated bridge target, do not edit */

#ifndef DESTACK_REPOSITORY_REVISION_GENERATED_H
#define DESTACK_REPOSITORY_REVISION_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackRevision {
    char *id;
} DestackRevision;

typedef struct DestackRevisionArray {
    DestackRevision *ptr;
    size_t len;
} DestackRevisionArray;

typedef struct DestackOptionalRevision {
    bool is_some;
    DestackRevision value;
} DestackOptionalRevision;

void destack_revision_destroy(DestackRevision *value);
void destack_revision_array_destroy(DestackRevisionArray array);

#ifdef __cplusplus
}
#endif

#endif
