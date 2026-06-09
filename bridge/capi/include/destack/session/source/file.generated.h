/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_SOURCE_FILE_GENERATED_H
#define DESTACK_SESSION_SOURCE_FILE_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/module.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackFileChangeKind {
    DESTACK_FILE_CHANGE_KIND_SOURCE = 0,
    DESTACK_FILE_CHANGE_KIND_CONFIG = 1,
} DestackFileChangeKind;

typedef struct DestackFileChangeKindArray {
    DestackFileChangeKind *ptr;
    size_t len;
} DestackFileChangeKindArray;

typedef struct DestackOptionalFileChangeKind {
    bool is_some;
    DestackFileChangeKind value;
} DestackOptionalFileChangeKind;

typedef struct DestackFileChange {
    char *path;
    char *uri;
    DestackFileChangeKind kind;
    bool is_removed;
    DestackOptionalModuleId module_id;
} DestackFileChange;

typedef struct DestackFileChangeArray {
    DestackFileChange *ptr;
    size_t len;
} DestackFileChangeArray;

typedef struct DestackOptionalFileChange {
    bool is_some;
    DestackFileChange value;
} DestackOptionalFileChange;

void destack_file_change_kind_destroy(DestackFileChangeKind *value);
void destack_file_change_kind_array_destroy(DestackFileChangeKindArray array);
void destack_file_change_destroy(DestackFileChange *value);
void destack_file_change_array_destroy(DestackFileChangeArray array);

#ifdef __cplusplus
}
#endif

#endif
