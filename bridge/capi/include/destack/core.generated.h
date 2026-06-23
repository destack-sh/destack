/* generated bridge target, do not edit */

#ifndef DESTACK_CORE_GENERATED_H
#define DESTACK_CORE_GENERATED_H

#include "destack/core.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackArtifactKey DestackArtifactKey;

typedef struct DestackByteArray {
    uint8_t *ptr;
    size_t len;
} DestackByteArray;

typedef struct DestackU128 {
    uint64_t high;
    uint64_t low;
} DestackU128;

typedef struct DestackStringArray {
    char **ptr;
    size_t len;
} DestackStringArray;

typedef struct DestackOptionalString {
    bool is_some;
    char *value;
} DestackOptionalString;

void destack_byte_array_destroy(DestackByteArray array);
void destack_string_array_destroy(DestackStringArray array);
void destack_optional_string_destroy(DestackOptionalString value);

#ifdef __cplusplus
}
#endif

#endif
