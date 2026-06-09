/* generated bridge target, do not edit */

#ifndef DESTACK_SOURCE_FILE_GENERATED_H
#define DESTACK_SOURCE_FILE_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

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

typedef struct DestackFileContentId {
    char *id;
} DestackFileContentId;

typedef struct DestackFileContentIdArray {
    DestackFileContentId *ptr;
    size_t len;
} DestackFileContentIdArray;

typedef struct DestackOptionalFileContentId {
    bool is_some;
    DestackFileContentId value;
} DestackOptionalFileContentId;

typedef enum DestackFileContentKind {
    DESTACK_FILE_CONTENT_KIND_TEXT = 0,
    DESTACK_FILE_CONTENT_KIND_BINARY = 1,
} DestackFileContentKind;

typedef struct DestackFileContent {
    DestackFileContentKind kind;
    char *text_content;
    DestackByteArray binary_content;
} DestackFileContent;

typedef struct DestackFileContentArray {
    DestackFileContent *ptr;
    size_t len;
} DestackFileContentArray;

typedef struct DestackOptionalFileContent {
    bool is_some;
    DestackFileContent value;
} DestackOptionalFileContent;

void destack_file_id_destroy(DestackFileId *value);
void destack_file_id_array_destroy(DestackFileIdArray array);
void destack_file_content_id_destroy(DestackFileContentId *value);
void destack_file_content_id_array_destroy(DestackFileContentIdArray array);
void destack_file_content_destroy(DestackFileContent *value);
void destack_file_content_array_destroy(DestackFileContentArray array);

#ifdef __cplusplus
}
#endif

#endif
