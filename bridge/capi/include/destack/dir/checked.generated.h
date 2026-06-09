/* generated bridge target, do not edit */

#ifndef DESTACK_DIR_CHECKED_GENERATED_H
#define DESTACK_DIR_CHECKED_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/source/component.generated.h"
#include "destack/source/module.generated.h"
#include "destack/source/profile.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackDirChecked {
    DestackArtifactVersion version;
    DestackModuleId module;
    DestackProfileId profile;
    DestackComponentId component;
    DestackModuleId entry;
} DestackDirChecked;

typedef struct DestackDirCheckedArray {
    DestackDirChecked *ptr;
    size_t len;
} DestackDirCheckedArray;

typedef struct DestackOptionalDirChecked {
    bool is_some;
    DestackDirChecked value;
} DestackOptionalDirChecked;

void destack_dir_checked_destroy(DestackDirChecked *value);
void destack_dir_checked_array_destroy(DestackDirCheckedArray array);

#ifdef __cplusplus
}
#endif

#endif
