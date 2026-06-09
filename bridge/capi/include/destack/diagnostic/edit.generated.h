/* generated bridge target, do not edit */

#ifndef DESTACK_DIAGNOSTIC_EDIT_GENERATED_H
#define DESTACK_DIAGNOSTIC_EDIT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/file.generated.h"
#include "destack/source/span.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackReplacement {
    DestackSpan span;
    char *new_text;
} DestackReplacement;

typedef struct DestackReplacementArray {
    DestackReplacement *ptr;
    size_t len;
} DestackReplacementArray;

typedef struct DestackOptionalReplacement {
    bool is_some;
    DestackReplacement value;
} DestackOptionalReplacement;

typedef struct DestackFilePatch {
    DestackFileId file;
    DestackReplacementArray replacements;
} DestackFilePatch;

typedef struct DestackFilePatchArray {
    DestackFilePatch *ptr;
    size_t len;
} DestackFilePatchArray;

typedef struct DestackOptionalFilePatch {
    bool is_some;
    DestackFilePatch value;
} DestackOptionalFilePatch;

typedef struct DestackBatchEdit {
    DestackFilePatchArray files;
} DestackBatchEdit;

typedef struct DestackBatchEditArray {
    DestackBatchEdit *ptr;
    size_t len;
} DestackBatchEditArray;

typedef struct DestackOptionalBatchEdit {
    bool is_some;
    DestackBatchEdit value;
} DestackOptionalBatchEdit;

void destack_replacement_destroy(DestackReplacement *value);
void destack_replacement_array_destroy(DestackReplacementArray array);
void destack_file_patch_destroy(DestackFilePatch *value);
void destack_file_patch_array_destroy(DestackFilePatchArray array);
void destack_batch_edit_destroy(DestackBatchEdit *value);
void destack_batch_edit_array_destroy(DestackBatchEditArray array);

#ifdef __cplusplus
}
#endif

#endif
