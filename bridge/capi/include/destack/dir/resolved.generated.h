/* generated bridge target, do not edit */

#ifndef DESTACK_DIR_RESOLVED_GENERATED_H
#define DESTACK_DIR_RESOLVED_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/source/module.generated.h"
#include "destack/source/profile.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackDirResolved {
    DestackArtifactVersion version;
    DestackModuleId module;
    DestackProfileId profile;
} DestackDirResolved;

typedef struct DestackDirResolvedArray {
    DestackDirResolved *ptr;
    size_t len;
} DestackDirResolvedArray;

typedef struct DestackOptionalDirResolved {
    bool is_some;
    DestackDirResolved value;
} DestackOptionalDirResolved;

void destack_dir_resolved_destroy(DestackDirResolved *value);
void destack_dir_resolved_array_destroy(DestackDirResolvedArray array);

#ifdef __cplusplus
}
#endif

#endif
