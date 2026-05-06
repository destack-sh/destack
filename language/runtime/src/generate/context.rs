use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactPinSet, ArtifactStore, ArtifactVersion, DirChecked, DirDeclared,
    LanguageEnvironment, LibraryEnvironment,
};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{Module, PinnedRevision, ProfileId, Repository, RepositoryError, Revision};

/// DIR surface used by runtime generation.
#[derive(Debug, Clone)]
pub(crate) struct GeneratorDir {
    /// The declared DIR artifact.
    declared: Arc<DirDeclared>,
    /// The checked DIR artifact.
    checked: Arc<DirChecked>,
}

impl GeneratorDir {
    /// Return the declared DIR tree.
    pub(crate) fn tree(&self) -> &dir::Tree {
        &self.declared.tree
    }

    /// Return the declared DIR symbol table.
    pub(crate) fn symbols(&self) -> &dir::SymbolTable {
        &self.declared.symbols
    }

    /// Return the checked DIR type table.
    pub(crate) fn types(&self) -> &dir::TypeTable {
        &self.checked.types
    }
}

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

    /// Return one retained DIR surface for runtime generation.
    pub(crate) fn dir(&self, module_id: ModuleId, profile_id: ProfileId) -> GeneratorDir {
        let declared = self
            .current_artifact(
                ArtifactKey::dir_declared(module_id, profile_id),
                |artifacts, version| artifacts.dir_declared(version),
            )
            .unwrap_or_else(|| panic!("missing declared DIR artifact for module {module_id:?}"));
        let checked = self
            .current_artifact(
                ArtifactKey::dir_checked(module_id, profile_id),
                |artifacts, version| artifacts.dir_checked(version),
            )
            .unwrap_or_else(|| panic!("missing checked DIR artifact for module {module_id:?}"));

        GeneratorDir { declared, checked }
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
