/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_VERSION_GENERATED_H
#define DESTACK_ARTIFACT_VERSION_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/key.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackArtifactVersion {
    DestackArtifactKey *key;
    char *fingerprint;
} DestackArtifactVersion;

typedef struct DestackArtifactVersionArray {
    DestackArtifactVersion *ptr;
    size_t len;
} DestackArtifactVersionArray;

typedef struct DestackOptionalArtifactVersion {
    bool is_some;
    DestackArtifactVersion value;
} DestackOptionalArtifactVersion;

void destack_artifact_version_destroy(DestackArtifactVersion *value);
void destack_artifact_version_array_destroy(DestackArtifactVersionArray array);

#ifdef __cplusplus
}
#endif

#endif
