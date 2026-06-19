/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_FILE_GENERATED_H
#define DESTACK_SOURCE_FILE_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackContentId {
    char *id;
} DestackContentId;

typedef struct DestackContentIdArray {
    DestackContentId *ptr;
    size_t len;
} DestackContentIdArray;

typedef struct DestackOptionalContentId {
    bool is_some;
    DestackContentId value;
} DestackOptionalContentId;

typedef struct DestackFileId {
    char *id;
} DestackFileId;

typedef struct DestackFileIdArray {
    DestackFileId *ptr;
    size_t len;
} DestackFileIdArray;

typedef struct DestackOptionalFileId {
    bool is_some;
    DestackFileId value;
} DestackOptionalFileId;

typedef enum DestackContentKind {
    DESTACK_CONTENT_KIND_TEXT = 0,
    DESTACK_CONTENT_KIND_BINARY = 1,
} DestackContentKind;

typedef struct DestackContent {
    DestackContentKind kind;
    char *text_content;
    DestackByteArray binary_content;
} DestackContent;

typedef struct DestackContentArray {
    DestackContent *ptr;
    size_t len;
} DestackContentArray;

typedef struct DestackOptionalContent {
    bool is_some;
    DestackContent value;
} DestackOptionalContent;

void destack_content_id_destroy(DestackContentId *value);
void destack_content_id_array_destroy(DestackContentIdArray array);
void destack_file_id_destroy(DestackFileId *value);
void destack_file_id_array_destroy(DestackFileIdArray array);
void destack_content_destroy(DestackContent *value);
void destack_content_array_destroy(DestackContentArray array);

#ifdef __cplusplus
}
#endif

#endif
