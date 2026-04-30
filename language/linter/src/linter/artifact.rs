use std::sync::Arc;

use destack_artifact::{
    AmbientEnvironment, ArtifactKey, ArtifactVersion, Ast, DirChecked, DirDeclared, DirExported,
    LanguageEnvironment,
};
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Repository, Revision};

/// Return one recorded artifact version.
pub(crate) fn artifact_version(
    repository: &Repository,
    revision: Revision,
    artifact_key: ArtifactKey,
) -> Option<ArtifactVersion> {
    repository
        .artifact_version(revision, &artifact_key)
        .ok()
        .flatten()
}

/// Read one AST artifact.
pub(crate) fn read_ast(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> Option<Arc<Ast>> {
    let key = ArtifactKey::ast(module_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().ast(&version)
}

/// Read one declared DIR artifact.
pub(crate) fn read_dir_declared(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<Arc<DirDeclared>> {
    let key = ArtifactKey::dir_declared(module_id, profile_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().dir_declared(&version)
}

/// Read one exported DIR artifact.
pub(crate) fn read_dir_exported(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<Arc<DirExported>> {
    let key = ArtifactKey::dir_exported(module_id, profile_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().dir_exported(&version)
}

/// Read one checked DIR artifact.
pub(crate) fn read_dir_checked(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<Arc<DirChecked>> {
    let key = ArtifactKey::dir_checked(module_id, profile_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().dir_checked(&version)
}

/// Read one language environment artifact.
pub(crate) fn read_language_environment(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
) -> Option<Arc<LanguageEnvironment>> {
    let key = ArtifactKey::language_environment(profile_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().language_environment(&version)
}

/// Read one ambient environment artifact.
pub(crate) fn read_ambient_environment(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
) -> Option<Arc<AmbientEnvironment>> {
    let key = ArtifactKey::ambient_environment(profile_id);
    let version = artifact_version(repository, revision, key)?;

    repository.artifact_store().ambient_environment(&version)
}
