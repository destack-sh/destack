use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactVersion, DirBound,
    DirChecked, DirExpanded, DirExported, DirImported, DirParsed, GlobalEnvironment,
};
use destack_source::{DiagnosticCollection, ModuleId, ProfileId};

use crate::repository::{Repository, RepositoryError, Revision};

/// Revision-scoped cache for committed artifacts.
#[derive(Debug)]
pub struct ArtifactCache {
    /// The repository that owns artifact bindings.
    repository: Arc<Repository>,
    /// The revision that owns artifact bindings.
    revision: Revision,
    /// Parsed DIR artifacts by module.
    dir_parsed: DashMap<ModuleId, Arc<DirParsed>>,
    /// Bound DIR artifacts by module and profile.
    dir_bound: DashMap<(ModuleId, ProfileId), Arc<DirBound>>,
    /// Imported DIR artifacts by module and profile.
    dir_imported: DashMap<(ModuleId, ProfileId), Arc<DirImported>>,
    /// Expanded DIR artifacts by module and profile.
    dir_expanded: DashMap<(ModuleId, ProfileId), Arc<DirExpanded>>,
    /// Exported DIR artifacts by module and profile.
    dir_exported: DashMap<(ModuleId, ProfileId), Arc<DirExported>>,
    /// Checked DIR artifacts by module and profile.
    dir_checked: DashMap<(ModuleId, ProfileId), Arc<DirChecked>>,
    /// Global environments by profile.
    global_environment: DashMap<ProfileId, Arc<GlobalEnvironment>>,
}

impl ArtifactCache {
    /// Create a cache for one repository revision.
    pub fn new(repository: Arc<Repository>, revision: Revision) -> Self {
        Self {
            repository,
            revision,
            dir_parsed: DashMap::new(),
            dir_bound: DashMap::new(),
            dir_imported: DashMap::new(),
            dir_expanded: DashMap::new(),
            dir_exported: DashMap::new(),
            dir_checked: DashMap::new(),
            global_environment: DashMap::new(),
        }
    }

    /// Return the recorded artifact version for one key.
    pub fn version(&self, key: ArtifactKey) -> Option<ArtifactVersion> {
        self.repository
            .artifact_version(self.revision, &key)
            .ok()
            .flatten()
    }

    /// Read one parsed DIR artifact.
    pub fn dir_parsed(&self, module_id: ModuleId) -> Option<Arc<DirParsed>> {
        self.read_cached(
            &self.dir_parsed,
            module_id,
            ArtifactKey::dir_parsed(module_id),
            |version| self.repository.artifact_store().dir_parsed(version),
        )
    }

    /// Read one bound DIR artifact.
    pub fn dir_bound(&self, module_id: ModuleId, profile_id: ProfileId) -> Option<Arc<DirBound>> {
        let key = (module_id, profile_id);
        let artifact_key = ArtifactKey::dir_bound(module_id, profile_id);

        self.read_cached(&self.dir_bound, key, artifact_key, |version| {
            self.repository.artifact_store().dir_bound(version)
        })
    }

    /// Read one imported DIR artifact.
    pub fn dir_imported(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirImported>> {
        let key = (module_id, profile_id);
        let artifact_key = ArtifactKey::dir_imported(module_id, profile_id);

        self.read_cached(&self.dir_imported, key, artifact_key, |version| {
            self.repository.artifact_store().dir_imported(version)
        })
    }

    /// Read one expanded DIR artifact.
    pub fn dir_expanded(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirExpanded>> {
        let key = (module_id, profile_id);
        let artifact_key = ArtifactKey::dir_expanded(module_id, profile_id);

        self.read_cached(&self.dir_expanded, key, artifact_key, |version| {
            self.repository.artifact_store().dir_expanded(version)
        })
    }

    /// Read one exported DIR artifact.
    pub fn dir_exported(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirExported>> {
        let key = (module_id, profile_id);
        let artifact_key = ArtifactKey::dir_exported(module_id, profile_id);

        self.read_cached(&self.dir_exported, key, artifact_key, |version| {
            self.repository.artifact_store().dir_exported(version)
        })
    }

    /// Read one checked DIR artifact.
    pub fn dir_checked(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirChecked>> {
        let key = (module_id, profile_id);
        let artifact_key = ArtifactKey::dir_checked(module_id, profile_id);

        self.read_cached(&self.dir_checked, key, artifact_key, |version| {
            self.repository.artifact_store().dir_checked(version)
        })
    }

    /// Read one global environment artifact.
    pub fn global_environment(&self, profile_id: ProfileId) -> Option<Arc<GlobalEnvironment>> {
        self.read_cached(
            &self.global_environment,
            profile_id,
            ArtifactKey::global_environment(profile_id),
            |version| self.repository.artifact_store().global_environment(version),
        )
    }

    /// Read and cache one artifact payload.
    fn read_cached<K, T>(
        &self,
        cache: &DashMap<K, Arc<T>>,
        key: K,
        artifact_key: ArtifactKey,
        load: impl FnOnce(&ArtifactVersion) -> Option<Arc<T>>,
    ) -> Option<Arc<T>>
    where
        K: Copy + Eq + Hash,
    {
        // return cached payload
        if let Some(payload) = cache.get(&key).map(|payload| payload.clone()) {
            return Some(payload);
        }

        // load committed payload
        let version = self.version(artifact_key)?;
        let payload = load(&version)?;

        // retain payload for this cache lifetime
        cache.insert(key, payload.clone());

        Some(payload)
    }
}

impl Repository {
    /// Return the recorded artifact version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let _revision = self.revision(revision)?;

        Ok(self
            .artifact_versions
            .get(&(revision, *artifact_key))
            .map(|version| *version.value()))
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store payload before exposing the revision binding
        self.artifact_store()
            .publish(version, payload, dependencies, diagnostics);
        self.artifact_versions
            .insert((revision, version.key), version);

        Ok(())
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store failure before exposing the revision binding
        self.artifact_store()
            .fail(version, dependencies, diagnostics, failure);
        self.artifact_versions
            .insert((revision, version.key), version);

        Ok(())
    }

    /// Return diagnostics for one revision, optionally filtered to one artifact key.
    pub fn diagnostics(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let _revision = self.revision(revision)?;
        let mut diagnostics = DiagnosticCollection::new();

        // exact artifact
        if let Some(artifact_key) = artifact_key {
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                return Ok(diagnostics);
            };
            let artifact_diagnostics = self
                .artifact_store()
                .diagnostics(&version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version })?;

            diagnostics.merge_from(&artifact_diagnostics);

            return Ok(diagnostics);
        }

        // all artifacts in this revision
        for entry in self.artifact_versions.iter() {
            let ((entry_revision, _artifact_key), version) = entry.pair();
            if *entry_revision != revision {
                continue;
            }

            let artifact_diagnostics = self
                .artifact_store()
                .diagnostics(version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or_else(|| RepositoryError::MissingArtifact { version: *version })?;
            diagnostics.merge_from(&artifact_diagnostics);
        }

        Ok(diagnostics)
    }
}
