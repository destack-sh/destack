/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_PROFILE_GENERATED_H
#define DESTACK_SOURCE_PROFILE_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackProfileId {
    char *id;
} DestackProfileId;

typedef struct DestackProfileIdArray {
    DestackProfileId *ptr;
    size_t len;
} DestackProfileIdArray;

typedef struct DestackOptionalProfileId {
    bool is_some;
    DestackProfileId value;
} DestackOptionalProfileId;

void destack_profile_id_destroy(DestackProfileId *value);
void destack_profile_id_array_destroy(DestackProfileIdArray array);

#ifdef __cplusplus
}
#endif

#endif
