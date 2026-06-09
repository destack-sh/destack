/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_SIDECAR_GENERATED_H
#define DESTACK_ARTIFACT_SIDECAR_GENERATED_H

#include "destack/core.generated.h"
#include "destack/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackArtifactSidecarLabel {
    char *key;
    char *value;
} DestackArtifactSidecarLabel;

typedef struct DestackArtifactSidecarLabelArray {
    DestackArtifactSidecarLabel *ptr;
    size_t len;
} DestackArtifactSidecarLabelArray;

typedef struct DestackOptionalArtifactSidecarLabel {
    bool is_some;
    DestackArtifactSidecarLabel value;
} DestackOptionalArtifactSidecarLabel;

typedef struct DestackArtifactSidecar {
    char *name;
    DestackArtifactSidecarLabelArray labels;
    DestackFileContent content;
} DestackArtifactSidecar;

typedef struct DestackArtifactSidecarArray {
    DestackArtifactSidecar *ptr;
    size_t len;
} DestackArtifactSidecarArray;

typedef struct DestackOptionalArtifactSidecar {
    bool is_some;
    DestackArtifactSidecar value;
} DestackOptionalArtifactSidecar;

void destack_artifact_sidecar_label_destroy(DestackArtifactSidecarLabel *value);
void destack_artifact_sidecar_label_array_destroy(DestackArtifactSidecarLabelArray array);
void destack_artifact_sidecar_destroy(DestackArtifactSidecar *value);
void destack_artifact_sidecar_array_destroy(DestackArtifactSidecarArray array);

#ifdef __cplusplus
}
#endif

#endif
