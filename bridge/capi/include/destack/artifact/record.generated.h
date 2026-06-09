/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_RECORD_GENERATED_H
#define DESTACK_ARTIFACT_RECORD_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/dependency.generated.h"
#include "destack/artifact/sidecar.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/diagnostic/diagnostic.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DestackArtifactString {
    char *id;
    char *text;
} DestackArtifactString;

typedef struct DestackArtifactStringArray {
    DestackArtifactString *ptr;
    size_t len;
} DestackArtifactStringArray;

typedef struct DestackOptionalArtifactString {
    bool is_some;
    DestackArtifactString value;
} DestackOptionalArtifactString;

typedef struct DestackArtifactRecord {
    DestackArtifactVersion version;
    DestackByteArray image;
    DestackArtifactStringArray strings;
    DestackArtifactDependencyArray dependencies;
    DestackDiagnosticArray diagnostics;
    DestackArtifactSidecarArray sidecars;
} DestackArtifactRecord;

typedef struct DestackArtifactRecordArray {
    DestackArtifactRecord *ptr;
    size_t len;
} DestackArtifactRecordArray;

typedef struct DestackOptionalArtifactRecord {
    bool is_some;
    DestackArtifactRecord value;
} DestackOptionalArtifactRecord;

void destack_artifact_string_destroy(DestackArtifactString *value);
void destack_artifact_string_array_destroy(DestackArtifactStringArray array);
void destack_artifact_record_destroy(DestackArtifactRecord *value);
void destack_artifact_record_array_destroy(DestackArtifactRecordArray array);

#ifdef __cplusplus
}
#endif

#endif
