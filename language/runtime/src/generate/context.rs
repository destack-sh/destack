use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactPinSet, ArtifactStore, ArtifactVersion, DirPatched, DirResolved,
    LanguageEnvironment, LibraryEnvironment,
};
use destack_compiler::Compiler;
use destack_source::ModuleId;
use destack_workspace::{Module, PinnedRevision, ProfileId, Repository, RepositoryError, Revision};

/// Revision-scoped semantic context for runtime generation.
#[derive(Debug)]
pub(crate) struct GeneratorContext {
    /// The pinned repository revision for this generator run.
    pinned_revision: PinnedRevision,
    /// The retained live artifacts for this generator run.
    retained_artifacts: ArtifactPinSet,
}

impl GeneratorContext {
    /// Build one context from a repository and revision.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision) -> Self {
        let pinned_revision = repository
            .pin(revision)
            .unwrap_or_else(|error| panic!("failed to root runtime generator revision: {error}"));
        let retained_artifacts = ArtifactPinSet::new(repository.artifact_store().clone());

        Self {
            pinned_revision,
            retained_artifacts,
        }
    }

    /// Return the shared repository.
    pub(crate) fn repository(&self) -> &Repository {
        self.pinned_revision.repository()
    }

    /// Return the active revision.
    pub(crate) fn revision(&self) -> Revision {
        self.pinned_revision.revision()
    }

    /// Return the live artifact store for type formatting helpers.
    pub(crate) fn artifact_store(&self) -> &Arc<ArtifactStore> {
        self.retained_artifacts.store()
    }

    /// Return one module from the active revision.
    pub(crate) fn get(&self, module_id: ModuleId) -> Arc<Module> {
        self.pinned_revision
            .module(module_id)
            .unwrap_or_else(|error| panic!("failed to load module {module_id:?}: {error}"))
            .unwrap_or_else(|| {
                panic!(
                    "missing module {module_id:?} in revision {:?}",
                    self.revision()
                )
            })
    }

    /// Return one retained language environment for the active profile.
    pub(crate) fn language_environment(&self, profile_id: ProfileId) -> Arc<LanguageEnvironment> {
        self.current_artifact(
            ArtifactKey::language_environment(profile_id),
            |artifacts, version| artifacts.language_environment(version),
        )
        .unwrap_or_else(|| panic!("missing language environment for profile {profile_id:?}"))
    }

    /// Return one retained library environment for the active profile.
    pub(crate) fn library_environment(&self, profile_id: ProfileId) -> Arc<LibraryEnvironment> {
        self.current_artifact(
            ArtifactKey::library_environment(profile_id),
            |artifacts, version| artifacts.library_environment(version),
        )
        .unwrap_or_else(|| panic!("missing library environment for profile {profile_id:?}"))
    }

    /// Return one retained resolved DIR artifact.
    pub(crate) fn dir_resolved(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Arc<DirResolved> {
        self.current_artifact(
            ArtifactKey::dir_resolved(module_id, profile_id),
            |artifacts, version| artifacts.dir_resolved(version),
        )
        .unwrap_or_else(|| panic!("missing resolved dir artifact for module {module_id:?}"))
    }

    /// Return one retained patched DIR artifact.
    pub(crate) fn dir_patched(
        &self,
        _compiler: &Compiler,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Arc<DirPatched> {
        self.current_artifact(
            ArtifactKey::dir_patched(module_id, profile_id),
            |artifacts, version| artifacts.dir_patched(version),
        )
        .unwrap_or_else(|| panic!("missing patched dir artifact for module {module_id:?}"))
    }

    /// Return one retained current live artifact for the active revision.
    fn current_artifact<T>(
        &self,
        artifact_key: ArtifactKey,
        load: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        let version = self
            .artifact_version(&artifact_key)
            .unwrap_or_else(|error| panic!("failed to resolve artifact version: {error}"));
        let payload = load(self.artifact_store().as_ref(), &version)?;

        self.retained_artifacts.pin(version);

        Some(payload)
    }

    /// Return one exact artifact version for the active revision.
    fn artifact_version(
        &self,
        artifact_key: &ArtifactKey,
    ) -> Result<ArtifactVersion, RepositoryError> {
        Ok(self
            .repository()
            .artifact_version(self.revision(), artifact_key))
    }
}
