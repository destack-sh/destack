/* generated bridge target, do not edit */

#ifndef DESTACK_DIAGNOSTIC_EDIT_GENERATED_H
#define DESTACK_DIAGNOSTIC_EDIT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/file.generated.h"
#include "destack/source/span.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackPatch {
    DestackSpan span;
    char *new_text;
} DestackPatch;

typedef struct DestackPatchArray {
    DestackPatch *ptr;
    size_t len;
} DestackPatchArray;

typedef struct DestackOptionalPatch {
    bool is_some;
    DestackPatch value;
} DestackOptionalPatch;

typedef struct DestackFilePatch {
    DestackFileId file;
    DestackPatchArray patches;
} DestackFilePatch;

typedef struct DestackFilePatchArray {
    DestackFilePatch *ptr;
    size_t len;
} DestackFilePatchArray;

typedef struct DestackOptionalFilePatch {
    bool is_some;
    DestackFilePatch value;
} DestackOptionalFilePatch;

typedef struct DestackPatchSet {
    DestackFilePatchArray files;
} DestackPatchSet;

typedef struct DestackPatchSetArray {
    DestackPatchSet *ptr;
    size_t len;
} DestackPatchSetArray;

typedef struct DestackOptionalPatchSet {
    bool is_some;
    DestackPatchSet value;
} DestackOptionalPatchSet;

void destack_patch_destroy(DestackPatch *value);
void destack_patch_array_destroy(DestackPatchArray array);
void destack_file_patch_destroy(DestackFilePatch *value);
void destack_file_patch_array_destroy(DestackFilePatchArray array);
void destack_patch_set_destroy(DestackPatchSet *value);
void destack_patch_set_array_destroy(DestackPatchSetArray array);

#ifdef __cplusplus
}
#endif

#endif
