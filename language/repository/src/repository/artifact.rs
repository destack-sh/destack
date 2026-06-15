use std::collections::BTreeMap;
use std::sync::Arc;

use destack_artifact::{
    ArtifactCache, ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactRecord, ArtifactSidecar, ArtifactVersion, ComponentGraph, Data,
    DirBound, DirCheckedComponent, DirCheckedModule, DirElaborated, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, MirLowered,
    MirOptimized, MirVerified, ModuleIndex, ModuleLinted, ModuleOutput, ModuleQueryIndex,
    PackageIndex, PackageLinted, PackageOutput, ProductOutput, WorkspaceLinted,
    WorkspaceQueryIndex,
};
use destack_source::{
    ComponentId, DiagnosticCollection, ModuleId, PackageId, ProductId, ProfileId, TargetId,
};

use crate::provider::ProviderError;
use crate::repository::{Repository, RepositoryError, Revision};

/// Revision-bound read-only view over ready artifacts.
pub struct ArtifactReader<'a> {
    /// The repository that binds artifact versions to the revision.
    repository: &'a Repository,
    /// The pinned revision the reader resolves against.
    revision: Revision,
    /// The artifact cache that owns typed payloads.
    store: Arc<ArtifactCache>,
}

impl std::fmt::Debug for ArtifactReader<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactReader")
            .finish_non_exhaustive()
    }
}

impl<'a> ArtifactReader<'a> {
    /// Create a read-only reader for one repository revision.
    pub fn new(repository: &'a Repository, revision: Revision) -> Self {
        let store = repository.artifact_cache().clone();

        Self {
            repository,
            revision,
            store,
        }
    }

    /// Resolve one artifact to its exact ready version.
    fn version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        let version = self
            .repository
            .artifact_version(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;

        // a not yet built dependency reads as blocked
        version
            .filter(|version| matches!(self.store.outcome(version), Some(ArtifactOutcome::Ok)))
            .ok_or_else(|| ProviderError::blocked(artifact_key))
    }

    /// Read one collected typed artifact payload.
    fn read<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactCache, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        let version = self.version(artifact_key)?;
        let payload = get(&self.store, &version).ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one parsed DIR artifact.
    pub fn dir_parsed(&self, module: ModuleId) -> Result<Arc<DirParsed>, ProviderError> {
        self.read(ArtifactKey::dir_parsed(module), ArtifactCache::dir_parsed)
    }

    /// Read one data artifact.
    pub fn data(&self, module: ModuleId) -> Result<Arc<Data>, ProviderError> {
        self.read(ArtifactKey::data(module), ArtifactCache::data)
    }

    /// Read one global environment artifact.
    pub fn global_environment(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        self.read(
            ArtifactKey::global_environment(profile),
            ArtifactCache::global_environment,
        )
    }

    /// Read one dependency index artifact.
    pub fn package_index(&self, profile: ProfileId) -> Result<Arc<PackageIndex>, ProviderError> {
        self.read(
            ArtifactKey::package_index(profile),
            ArtifactCache::package_index,
        )
    }

    /// Read one module index artifact.
    pub fn module_index(&self, profile: ProfileId) -> Result<Arc<ModuleIndex>, ProviderError> {
        self.read(
            ArtifactKey::module_index(profile),
            ArtifactCache::module_index,
        )
    }

    /// Read one component graph artifact.
    pub fn component_graph(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<ComponentGraph>, ProviderError> {
        self.read(
            ArtifactKey::component_graph(profile),
            ArtifactCache::component_graph,
        )
    }

    /// Read one bound DIR artifact.
    pub fn dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        self.read(
            ArtifactKey::dir_bound(module, profile),
            ArtifactCache::dir_bound,
        )
    }

    /// Read one imported DIR artifact.
    pub fn dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        self.read(
            ArtifactKey::dir_imported(module, profile),
            ArtifactCache::dir_imported,
        )
    }

    /// Read one expanded DIR artifact.
    pub fn dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        self.read(
            ArtifactKey::dir_expanded(module, profile),
            ArtifactCache::dir_expanded,
        )
    }

    /// Read one exported DIR artifact.
    pub fn dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        self.read(
            ArtifactKey::dir_exported(module, profile),
            ArtifactCache::dir_exported,
        )
    }

    /// Read one resolved DIR artifact.
    pub fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        self.read(
            ArtifactKey::dir_resolved(module, profile),
            ArtifactCache::dir_resolved,
        )
    }

    /// Read one checked DIR module output.
    pub fn dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedModule>, ProviderError> {
        let version = self.version(ArtifactKey::dir_checked(module, profile))?;
        let checked = self
            .store
            .dir_checked(&version)
            .ok_or(ProviderError::Corrupt { version })?;
        let component = self.dir_checked_component(checked.entry, checked.component, profile)?;
        let entry = component.module(module).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked component {} does not contain module {module:?}",
                checked.component
            ))
        })?;

        Ok(Arc::new(entry.checked.clone()))
    }

    /// Read one checked DIR component artifact.
    pub fn dir_checked_component(
        &self,
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedComponent>, ProviderError> {
        self.read(
            ArtifactKey::dir_checked_component(entry, component, profile),
            ArtifactCache::dir_checked_component,
        )
    }

    /// Read one materialized DIR artifact.
    pub fn dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirMaterialized>, ProviderError> {
        self.read(
            ArtifactKey::dir_materialized(module, profile),
            ArtifactCache::dir_materialized,
        )
    }

    /// Read one elaborated DIR artifact.
    pub fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ProviderError> {
        self.read(
            ArtifactKey::dir_elaborated(module, profile),
            ArtifactCache::dir_elaborated,
        )
    }

    /// Read one lowered MIR artifact.
    pub fn mir_lowered(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        self.read(
            ArtifactKey::mir_lowered(module, profile, target),
            ArtifactCache::mir_lowered,
        )
    }

    /// Read one verified MIR artifact.
    pub fn mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirVerified>, ProviderError> {
        self.read(
            ArtifactKey::mir_verified(module, profile, target),
            ArtifactCache::mir_verified,
        )
    }

    /// Read one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        self.read(
            ArtifactKey::mir_optimized(module, profile, target),
            ArtifactCache::mir_optimized,
        )
    }

    /// Read one module query index artifact.
    pub fn module_query_index(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleQueryIndex>, ProviderError> {
        self.read(
            ArtifactKey::module_query_index(module, profile),
            ArtifactCache::module_query_index,
        )
    }

    /// Read one workspace query index artifact.
    pub fn workspace_query_index(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<WorkspaceQueryIndex>, ProviderError> {
        self.read(
            ArtifactKey::workspace_query_index(profile),
            ArtifactCache::workspace_query_index,
        )
    }

    /// Read one module output artifact.
    pub fn module_output(
        &self,
        module: ModuleId,
        target: TargetId,
    ) -> Result<Arc<ModuleOutput>, ProviderError> {
        self.read(
            ArtifactKey::module_output(module, target),
            ArtifactCache::module_output,
        )
    }

    /// Read one package output artifact.
    pub fn package_output(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<PackageOutput>, ProviderError> {
        self.read(
            ArtifactKey::package_output(package, target),
            ArtifactCache::package_output,
        )
    }

    /// Read one product output artifact.
    pub fn product_output(
        &self,
        package: PackageId,
        product: ProductId,
    ) -> Result<Arc<ProductOutput>, ProviderError> {
        self.read(
            ArtifactKey::product_output(package, product),
            ArtifactCache::product_output,
        )
    }

    /// Read one module lint marker artifact.
    pub fn module_linted(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleLinted>, ProviderError> {
        self.read(
            ArtifactKey::module_linted(module, profile),
            ArtifactCache::module_linted,
        )
    }

    /// Read one package lint marker artifact.
    pub fn package_linted(&self, package: PackageId) -> Result<Arc<PackageLinted>, ProviderError> {
        self.read(
            ArtifactKey::package_linted(package),
            ArtifactCache::package_linted,
        )
    }

    /// Read the workspace lint marker artifact.
    pub fn workspace_linted(&self) -> Result<Arc<WorkspaceLinted>, ProviderError> {
        self.read(
            ArtifactKey::workspace_linted(),
            ArtifactCache::workspace_linted,
        )
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

    /// Bind one already-stored artifact version to one revision.
    pub fn bind_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // the version must already carry a terminal outcome to be reusable
        if self.artifact_cache().outcome(&version).is_none() {
            return Err(RepositoryError::MissingArtifact { version });
        }

        self.artifact_versions
            .insert((revision, version.key), version);

        Ok(())
    }

    /// Load one ready artifact from the persistent artifact store when present.
    pub fn load_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<bool, RepositoryError> {
        let Some(record) = self.artifact_store().load(&version).map_err(|error| {
            RepositoryError::ArtifactStore {
                message: error.to_string(),
            }
        })?
        else {
            return Ok(false);
        };

        self.load_artifact_record(revision, record)?;

        Ok(true)
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        self.publish_ready_artifact(
            revision,
            version,
            payload,
            dependencies,
            diagnostics,
            sidecars,
        )?;
        self.store_artifact(version)?;

        Ok(())
    }

    /// Publish one loaded artifact record and bind its exact version to one revision.
    fn load_artifact_record(
        &self,
        revision: Revision,
        record: ArtifactRecord,
    ) -> Result<(), RepositoryError> {
        let payload = record
            .decode_payload()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;
        self.string_pool().ensure_all_from(&record.strings);

        self.publish_ready_artifact(
            revision,
            record.version,
            payload,
            record.dependencies,
            record.diagnostics,
            record.sidecars,
        )
    }

    /// Publish one ready payload without writing the persistent cache.
    fn publish_ready_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        let key = version.key;
        self.artifact_cache()
            .publish(version, payload, dependencies, diagnostics, sidecars);
        self.artifact_versions.insert((revision, key), version);

        Ok(())
    }

    /// Store one ready artifact in the persistent artifact store.
    fn store_artifact(&self, version: ArtifactVersion) -> Result<(), RepositoryError> {
        self.artifact_store()
            .store(&version, self.artifact_cache(), self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Load all content ids referenced by one artifact payload.
    fn load_artifact_contents(&self, payload: &ArtifactPayload) -> Result<(), RepositoryError> {
        for content in payload.content_ids() {
            let _ = self.content(content)?;
        }

        Ok(())
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store failure before exposing the revision binding
        let key = version.key;

        self.artifact_cache()
            .fail(version, dependencies, diagnostics, sidecars, failure);
        self.artifact_versions.insert((revision, key), version);

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
                .artifact_cache()
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
                .artifact_cache()
                .diagnostics(version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version: *version })?;
            diagnostics.merge_from(&artifact_diagnostics);
        }

        Ok(diagnostics)
    }

    /// Return sidecars for one revision-scoped artifact key.
    pub fn artifact_sidecars(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Arc<[ArtifactSidecar]>, RepositoryError> {
        let _revision = self.revision(revision)?;
        let Some(version) = self.artifact_version(revision, &artifact_key)? else {
            return Ok(Arc::from([]));
        };

        self.artifact_cache()
            .sidecars(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }

    /// Return one sidecar by exact name and label set.
    pub fn artifact_sidecar(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        name: &str,
        labels: &BTreeMap<String, String>,
    ) -> Result<Option<ArtifactSidecar>, RepositoryError> {
        let sidecars = self.artifact_sidecars(revision, artifact_key)?;

        Ok(sidecars
            .iter()
            .find(|sidecar| sidecar.matches(name, labels))
            .cloned())
    }
}
