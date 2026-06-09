/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_DEPENDENCY_GENERATED_H
#define DESTACK_ARTIFACT_DEPENDENCY_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/source/file.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackArtifactPathState {
    DESTACK_ARTIFACT_PATH_STATE_MISSING = 0,
    DESTACK_ARTIFACT_PATH_STATE_FILE = 1,
    DESTACK_ARTIFACT_PATH_STATE_DIRECTORY = 2,
    DESTACK_ARTIFACT_PATH_STATE_SYMLINK = 3,
    DESTACK_ARTIFACT_PATH_STATE_OTHER = 4,
} DestackArtifactPathState;

typedef struct DestackArtifactPathStateArray {
    DestackArtifactPathState *ptr;
    size_t len;
} DestackArtifactPathStateArray;

typedef struct DestackOptionalArtifactPathState {
    bool is_some;
    DestackArtifactPathState value;
} DestackOptionalArtifactPathState;

typedef struct DestackArtifactDirectoryEntry {
    DestackFileId path;
    DestackArtifactPathState state;
} DestackArtifactDirectoryEntry;

typedef struct DestackArtifactDirectoryEntryArray {
    DestackArtifactDirectoryEntry *ptr;
    size_t len;
} DestackArtifactDirectoryEntryArray;

typedef struct DestackOptionalArtifactDirectoryEntry {
    bool is_some;
    DestackArtifactDirectoryEntry value;
} DestackOptionalArtifactDirectoryEntry;

typedef enum DestackArtifactSourceDependencyKind {
    DESTACK_ARTIFACT_SOURCE_DEPENDENCY_KIND_PATH_STATE = 0,
    DESTACK_ARTIFACT_SOURCE_DEPENDENCY_KIND_DIRECTORY_ENTRIES = 1,
    DESTACK_ARTIFACT_SOURCE_DEPENDENCY_KIND_FILE_CONTENT = 2,
} DestackArtifactSourceDependencyKind;

typedef struct DestackArtifactSourceDependency {
    DestackArtifactSourceDependencyKind kind;
    DestackFileId path;
    DestackArtifactPathState state;
    DestackFileId directory;
    DestackArtifactDirectoryEntryArray entries;
    DestackFileId file;
    DestackFileContentId content;
} DestackArtifactSourceDependency;

typedef struct DestackArtifactSourceDependencyArray {
    DestackArtifactSourceDependency *ptr;
    size_t len;
} DestackArtifactSourceDependencyArray;

typedef struct DestackOptionalArtifactSourceDependency {
    bool is_some;
    DestackArtifactSourceDependency value;
} DestackOptionalArtifactSourceDependency;

typedef enum DestackArtifactDependencyKind {
    DESTACK_ARTIFACT_DEPENDENCY_KIND_ARTIFACT = 0,
    DESTACK_ARTIFACT_DEPENDENCY_KIND_SOURCE = 1,
} DestackArtifactDependencyKind;

typedef struct DestackArtifactDependency {
    DestackArtifactDependencyKind kind;
    DestackArtifactVersion version;
    DestackArtifactSourceDependency dependency;
} DestackArtifactDependency;

typedef struct DestackArtifactDependencyArray {
    DestackArtifactDependency *ptr;
    size_t len;
} DestackArtifactDependencyArray;

typedef struct DestackOptionalArtifactDependency {
    bool is_some;
    DestackArtifactDependency value;
} DestackOptionalArtifactDependency;

void destack_artifact_path_state_destroy(DestackArtifactPathState *value);
void destack_artifact_path_state_array_destroy(DestackArtifactPathStateArray array);
void destack_artifact_directory_entry_destroy(DestackArtifactDirectoryEntry *value);
void destack_artifact_directory_entry_array_destroy(DestackArtifactDirectoryEntryArray array);
void destack_artifact_source_dependency_destroy(DestackArtifactSourceDependency *value);
void destack_artifact_source_dependency_array_destroy(DestackArtifactSourceDependencyArray array);
void destack_artifact_dependency_destroy(DestackArtifactDependency *value);
void destack_artifact_dependency_array_destroy(DestackArtifactDependencyArray array);

#ifdef __cplusplus
}
#endif

#endif
