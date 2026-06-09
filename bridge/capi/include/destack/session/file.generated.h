/* generated bridge target, do not edit */

#ifndef DESTACK_SESSION_FILE_GENERATED_H
#define DESTACK_SESSION_FILE_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackSessionFile {
    char *path;
} DestackSessionFile;

typedef struct DestackSessionFileArray {
    DestackSessionFile *ptr;
    size_t len;
} DestackSessionFileArray;

typedef struct DestackOptionalSessionFile {
    bool is_some;
    DestackSessionFile value;
} DestackOptionalSessionFile;

void destack_session_file_destroy(DestackSessionFile *value);
void destack_session_file_array_destroy(DestackSessionFileArray array);

#ifdef __cplusplus
}
#endif

#endif
