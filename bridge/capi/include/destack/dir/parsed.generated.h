/* generated bridge target, do not edit */

#ifndef DESTACK_DIR_PARSED_GENERATED_H
#define DESTACK_DIR_PARSED_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/source/module.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackDirParsed {
    DestackArtifactVersion version;
    DestackModuleId module;
} DestackDirParsed;

typedef struct DestackDirParsedArray {
    DestackDirParsed *ptr;
    size_t len;
} DestackDirParsedArray;

typedef struct DestackOptionalDirParsed {
    bool is_some;
    DestackDirParsed value;
} DestackOptionalDirParsed;

void destack_dir_parsed_destroy(DestackDirParsed *value);
void destack_dir_parsed_array_destroy(DestackDirParsedArray array);

#ifdef __cplusplus
}
#endif

#endif
