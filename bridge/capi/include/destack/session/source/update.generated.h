/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_SOURCE_UPDATE_GENERATED_H
#define DESTACK_SESSION_SOURCE_UPDATE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/repository/revision.generated.h"
#include "destack/session/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackFileUpdateResult {
    DestackRevision before;
    DestackRevision after;
    DestackFileChangeArray files;
} DestackFileUpdateResult;

typedef struct DestackFileUpdateResultArray {
    DestackFileUpdateResult *ptr;
    size_t len;
} DestackFileUpdateResultArray;

typedef struct DestackOptionalFileUpdateResult {
    bool is_some;
    DestackFileUpdateResult value;
} DestackOptionalFileUpdateResult;

typedef struct DestackTextRange {
    uint32_t start;
    uint32_t end;
} DestackTextRange;

typedef struct DestackTextRangeArray {
    DestackTextRange *ptr;
    size_t len;
} DestackTextRangeArray;

typedef struct DestackOptionalTextRange {
    bool is_some;
    DestackTextRange value;
} DestackOptionalTextRange;

typedef struct DestackTextEdit {
    DestackTextRange range;
    char *text;
} DestackTextEdit;

typedef struct DestackTextEditArray {
    DestackTextEdit *ptr;
    size_t len;
} DestackTextEditArray;

typedef struct DestackOptionalTextEdit {
    bool is_some;
    DestackTextEdit value;
} DestackOptionalTextEdit;

void destack_file_update_result_destroy(DestackFileUpdateResult *value);
void destack_file_update_result_array_destroy(DestackFileUpdateResultArray array);
void destack_text_range_destroy(DestackTextRange *value);
void destack_text_range_array_destroy(DestackTextRangeArray array);
void destack_text_edit_destroy(DestackTextEdit *value);
void destack_text_edit_array_destroy(DestackTextEditArray array);

#ifdef __cplusplus
}
#endif

#endif
