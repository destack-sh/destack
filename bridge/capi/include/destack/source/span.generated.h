/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_SPAN_GENERATED_H
#define DESTACK_SOURCE_SPAN_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackSpan {
    DestackFileId file;
    uint32_t start;
    uint32_t end;
} DestackSpan;

typedef struct DestackSpanArray {
    DestackSpan *ptr;
    size_t len;
} DestackSpanArray;

typedef struct DestackOptionalSpan {
    bool is_some;
    DestackSpan value;
} DestackOptionalSpan;

void destack_span_destroy(DestackSpan *value);
void destack_span_array_destroy(DestackSpanArray array);

#ifdef __cplusplus
}
#endif

#endif
