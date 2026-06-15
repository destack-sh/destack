/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_PRODUCT_GENERATED_H
#define DESTACK_SOURCE_PRODUCT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/package.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackProductId {
    DestackPackageId package;
    char *key;
} DestackProductId;

typedef struct DestackProductIdArray {
    DestackProductId *ptr;
    size_t len;
} DestackProductIdArray;

typedef struct DestackOptionalProductId {
    bool is_some;
    DestackProductId value;
} DestackOptionalProductId;

void destack_product_id_destroy(DestackProductId *value);
void destack_product_id_array_destroy(DestackProductIdArray array);

#ifdef __cplusplus
}
#endif

#endif
