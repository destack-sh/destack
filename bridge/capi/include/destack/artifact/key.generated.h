/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_KEY_GENERATED_H
#define DESTACK_ARTIFACT_KEY_GENERATED_H

#include "destack/core.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

void destack_artifact_key_destroy(DestackArtifactKey *value);
DestackStatus destack_artifact_key_dir_parsed(
    DestackModuleId module,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_data(
    DestackModuleId module,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_global_environment(
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_package_index(
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_module_index(
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_component_graph(
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_bound(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_imported(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_expanded(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_exported(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_resolved(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_checked_component(
    DestackModuleId entry,
    DestackComponentId component,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_checked(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_materialized(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_dir_elaborated(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_mir_lowered(
    DestackModuleId module,
    DestackProfileId profile,
    DestackTargetId target,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_mir_verified(
    DestackModuleId module,
    DestackProfileId profile,
    DestackTargetId target,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_mir_optimized(
    DestackModuleId module,
    DestackProfileId profile,
    DestackTargetId target,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_module_query_index(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_workspace_query_index(
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_module_output(
    DestackModuleId module,
    DestackTargetId target,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_package_output(
    DestackPackageId package,
    DestackTargetId target,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_module_linted(
    DestackModuleId module,
    DestackProfileId profile,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_package_linted(
    DestackPackageId package,
    DestackArtifactKey **out,
    DestackError **error
);
DestackStatus destack_artifact_key_workspace_linted(
    DestackArtifactKey **out,
    DestackError **error
);

#ifdef __cplusplus
}
#endif

#endif
