/* generated bridge target, do not edit */

#ifndef DESTACK_DIAGNOSTIC_EDIT_GENERATED_H
#define DESTACK_DIAGNOSTIC_EDIT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/file.generated.h"
#include "destack/source/span.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackEdit {
    DestackSpan span;
    char *new_text;
} DestackEdit;

typedef struct DestackEditArray {
    DestackEdit *ptr;
    size_t len;
} DestackEditArray;

typedef struct DestackOptionalEdit {
    bool is_some;
    DestackEdit value;
} DestackOptionalEdit;

typedef struct DestackFilePatch {
    DestackFileId file;
    DestackEditArray edits;
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

void destack_edit_destroy(DestackEdit *value);
void destack_edit_array_destroy(DestackEditArray array);
void destack_file_patch_destroy(DestackFilePatch *value);
void destack_file_patch_array_destroy(DestackFilePatchArray array);
void destack_batch_edit_destroy(DestackBatchEdit *value);
void destack_batch_edit_array_destroy(DestackBatchEditArray array);

#ifdef __cplusplus
}
#endif

#endif
