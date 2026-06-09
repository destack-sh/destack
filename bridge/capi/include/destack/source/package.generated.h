/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_PACKAGE_GENERATED_H
#define DESTACK_SOURCE_PACKAGE_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackPackageId {
    char *id;
} DestackPackageId;

typedef struct DestackPackageIdArray {
    DestackPackageId *ptr;
    size_t len;
} DestackPackageIdArray;

typedef struct DestackOptionalPackageId {
    bool is_some;
    DestackPackageId value;
} DestackOptionalPackageId;

void destack_package_id_destroy(DestackPackageId *value);
void destack_package_id_array_destroy(DestackPackageIdArray array);

#ifdef __cplusplus
}
#endif

#endif
