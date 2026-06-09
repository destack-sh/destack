/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_SOURCE_UPDATE_GENERATED_H
#define DESTACK_SESSION_SOURCE_UPDATE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/repository/revision.generated.h"
#include "destack/session/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackCommit {
    DestackRevision before;
    DestackRevision after;
    DestackChangeArray changes;
} DestackCommit;

typedef struct DestackCommitArray {
    DestackCommit *ptr;
    size_t len;
} DestackCommitArray;

typedef struct DestackOptionalCommit {
    bool is_some;
    DestackCommit value;
} DestackOptionalCommit;

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

void destack_commit_destroy(DestackCommit *value);
void destack_commit_array_destroy(DestackCommitArray array);
void destack_text_range_destroy(DestackTextRange *value);
void destack_text_range_array_destroy(DestackTextRangeArray array);
void destack_text_edit_destroy(DestackTextEdit *value);
void destack_text_edit_array_destroy(DestackTextEditArray array);

#ifdef __cplusplus
}
#endif

#endif
