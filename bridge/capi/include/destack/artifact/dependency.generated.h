/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_DEPENDENCY_GENERATED_H
#define DESTACK_ARTIFACT_DEPENDENCY_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackArtifactDependencyKind {
    DESTACK_ARTIFACT_DEPENDENCY_KIND_ARTIFACT = 0,
    DESTACK_ARTIFACT_DEPENDENCY_KIND_SOURCE = 1,
} DestackArtifactDependencyKind;

typedef struct DestackArtifactDependency {
    DestackArtifactDependencyKind kind;
    DestackArtifactVersion version;
    DestackFileId file;
    DestackContentId content;
} DestackArtifactDependency;

typedef struct DestackArtifactDependencyArray {
    DestackArtifactDependency *ptr;
    size_t len;
} DestackArtifactDependencyArray;

typedef struct DestackOptionalArtifactDependency {
    bool is_some;
    DestackArtifactDependency value;
} DestackOptionalArtifactDependency;

void destack_artifact_dependency_destroy(DestackArtifactDependency *value);
void destack_artifact_dependency_array_destroy(DestackArtifactDependencyArray array);

#ifdef __cplusplus
}
#endif

#endif
