use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactSidecar,
    ArtifactStore, ArtifactVersion, Data, DirBound, DirChecked, DirElaborated, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, MirLowered,
    MirOptimized, MirVerified, ModuleLinted, ModuleOutput, ModuleQueryIndex, PackageLinted,
    PackageOutput, WorkspaceLinted, WorkspaceQueryIndex,
};
use destack_source::{DiagnosticCollection, ModuleId, PackageId, ProfileId, TargetId};

use crate::provider::{ProviderContext, ProviderError};
use crate::repository::{Repository, RepositoryError, Revision};

/// Provider-scoped reader for required artifacts.
pub struct ArtifactReader<'a> {
    /// The provider attempt that owns dependency tracking.
    context: &'a dyn ProviderContext,
    /// The artifact store that owns typed payloads.
    store: Arc<ArtifactStore>,
}

impl std::fmt::Debug for ArtifactReader<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactReader")
            .finish_non_exhaustive()
    }
}

impl<'a> ArtifactReader<'a> {
    /// Create a provider-scoped reader.
    pub fn new(context: &'a dyn ProviderContext, store: Arc<ArtifactStore>) -> Self {
        Self { context, store }
    }

    /// Require one artifact without reading its payload.
    pub fn require(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        self.context.require(artifact_key)
    }

    /// Require several artifacts without reading their payloads.
    pub fn require_all(
        &self,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<ArtifactVersion>, ProviderError> {
        self.context.require_all(artifact_keys)
    }

    /// Require and read one typed artifact payload.
    fn read_required<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        // require exact artifact version
        let version = self.context.require(artifact_key)?;
        let payload = get(&self.store, &version).ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Require and read one parsed DIR artifact.
    pub fn dir_parsed(&self, module: ModuleId) -> Result<Arc<DirParsed>, ProviderError> {
        self.read_required(ArtifactKey::dir_parsed(module), ArtifactStore::dir_parsed)
    }

    /// Require and read one data artifact.
    pub fn data(&self, module: ModuleId) -> Result<Arc<Data>, ProviderError> {
        self.read_required(ArtifactKey::data(module), ArtifactStore::data)
    }

    /// Require and read one global environment artifact.
    pub fn global_environment(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        self.read_required(
            ArtifactKey::global_environment(profile),
            ArtifactStore::global_environment,
        )
    }

    /// Require and read one bound DIR artifact.
    pub fn dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_bound(module, profile),
            ArtifactStore::dir_bound,
        )
    }

    /// Require and read one imported DIR artifact.
    pub fn dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_imported(module, profile),
            ArtifactStore::dir_imported,
        )
    }

    /// Require and read one expanded DIR artifact.
    pub fn dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_expanded(module, profile),
            ArtifactStore::dir_expanded,
        )
    }

    /// Require and read one exported DIR artifact.
    pub fn dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_exported(module, profile),
            ArtifactStore::dir_exported,
        )
    }

    /// Require and read one resolved DIR artifact.
    pub fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_resolved(module, profile),
            ArtifactStore::dir_resolved,
        )
    }

    /// Require and read one checked DIR artifact.
    pub fn dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirChecked>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_checked(module, profile),
            ArtifactStore::dir_checked,
        )
    }

    /// Require and read one materialized DIR artifact.
    pub fn dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirMaterialized>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_materialized(module, profile),
            ArtifactStore::dir_materialized,
        )
    }

    /// Require and read one elaborated DIR artifact.
    pub fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ProviderError> {
        self.read_required(
            ArtifactKey::dir_elaborated(module, profile),
            ArtifactStore::dir_elaborated,
        )
    }

    /// Require and read one lowered MIR artifact.
    pub fn mir_lowered(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        self.read_required(
            ArtifactKey::mir_lowered(module, profile, target),
            ArtifactStore::mir_lowered,
        )
    }

    /// Require and read one verified MIR artifact.
    pub fn mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirVerified>, ProviderError> {
        self.read_required(
            ArtifactKey::mir_verified(module, profile, target),
            ArtifactStore::mir_verified,
        )
    }

    /// Require and read one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        self.read_required(
            ArtifactKey::mir_optimized(module, profile, target),
            ArtifactStore::mir_optimized,
        )
    }

    /// Require and read one module query index artifact.
    pub fn module_query_index(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleQueryIndex>, ProviderError> {
        self.read_required(
            ArtifactKey::module_query_index(module, profile),
            ArtifactStore::module_query_index,
        )
    }

    /// Require and read one workspace query index artifact.
    pub fn workspace_query_index(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<WorkspaceQueryIndex>, ProviderError> {
        self.read_required(
            ArtifactKey::workspace_query_index(profile),
            ArtifactStore::workspace_query_index,
        )
    }

    /// Require and read one module output artifact.
    pub fn module_output(
        &self,
        module: ModuleId,
        target: TargetId,
    ) -> Result<Arc<ModuleOutput>, ProviderError> {
        self.read_required(
            ArtifactKey::module_output(module, target),
            ArtifactStore::module_output,
        )
    }

    /// Require and read one package output artifact.
    pub fn package_output(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<PackageOutput>, ProviderError> {
        self.read_required(
            ArtifactKey::package_output(package, target),
            ArtifactStore::package_output,
        )
    }

    /// Require and read one module lint marker artifact.
    pub fn module_linted(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleLinted>, ProviderError> {
        self.read_required(
            ArtifactKey::module_linted(module, profile),
            ArtifactStore::module_linted,
        )
    }

    /// Require and read one package lint marker artifact.
    pub fn package_linted(&self, package: PackageId) -> Result<Arc<PackageLinted>, ProviderError> {
        self.read_required(
            ArtifactKey::package_linted(package),
            ArtifactStore::package_linted,
        )
    }

    /// Require and read the workspace lint marker artifact.
    pub fn workspace_linted(&self) -> Result<Arc<WorkspaceLinted>, ProviderError> {
        self.read_required(
            ArtifactKey::workspace_linted(),
            ArtifactStore::workspace_linted,
        )
    }
}

/// Revision-scoped cache for committed artifacts.
#[derive(Debug)]
pub struct ArtifactCache {
    /// The repository that owns artifact bindings.
    repository: Arc<Repository>,
    /// The revision that owns artifact bindings.
    revision: Revision,
    /// Parsed DIR artifacts by module.
    dir_parsed: DashMap<ModuleId, Arc<DirParsed>>,
    /// Parsed data artifacts by module.
    data: DashMap<ModuleId, Arc<Data>>,
    /// Global environments by profile.
    global_environment: DashMap<ProfileId, Arc<GlobalEnvironment>>,
    /// Bound DIR artifacts by module and profile.
    dir_bound: DashMap<(ModuleId, ProfileId), Arc<DirBound>>,
    /// Imported DIR artifacts by module and profile.
    dir_imported: DashMap<(ModuleId, ProfileId), Arc<DirImported>>,
    /// Expanded DIR artifacts by module and profile.
    dir_expanded: DashMap<(ModuleId, ProfileId), Arc<DirExpanded>>,
    /// Exported DIR artifacts by module and profile.
    dir_exported: DashMap<(ModuleId, ProfileId), Arc<DirExported>>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: DashMap<(ModuleId, ProfileId), Arc<DirResolved>>,
    /// Checked DIR artifacts by module and profile.
    dir_checked: DashMap<(ModuleId, ProfileId), Arc<DirChecked>>,
    /// Materialized DIR artifacts by module and profile.
    dir_materialized: DashMap<(ModuleId, ProfileId), Arc<DirMaterialized>>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: DashMap<(ModuleId, ProfileId), Arc<DirElaborated>>,
    /// Lowered MIR artifacts by module, profile, and target.
    mir_lowered: DashMap<(ModuleId, ProfileId, TargetId), Arc<MirLowered>>,
    /// Verified MIR artifacts by module, profile, and target.
    mir_verified: DashMap<(ModuleId, ProfileId, TargetId), Arc<MirVerified>>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: DashMap<(ModuleId, ProfileId, TargetId), Arc<MirOptimized>>,
    /// Module query indexes by module and profile.
    module_query_index: DashMap<(ModuleId, ProfileId), Arc<ModuleQueryIndex>>,
    /// Workspace query indexes by profile.
    workspace_query_index: DashMap<ProfileId, Arc<WorkspaceQueryIndex>>,
    /// Module outputs by module and target.
    module_output: DashMap<(ModuleId, TargetId), Arc<ModuleOutput>>,
    /// Package outputs by package and target.
    package_output: DashMap<(PackageId, TargetId), Arc<PackageOutput>>,
    /// Module lint markers by module and profile.
    module_linted: DashMap<(ModuleId, ProfileId), Arc<ModuleLinted>>,
    /// Package lint markers by package.
    package_linted: DashMap<PackageId, Arc<PackageLinted>>,
    /// Workspace lint marker.
    workspace_linted: DashMap<(), Arc<WorkspaceLinted>>,
}

impl ArtifactCache {
    /// Create a cache for one repository revision.
    pub fn new(repository: Arc<Repository>, revision: Revision) -> Self {
        Self {
            repository,
            revision,
            dir_parsed: DashMap::new(),
            data: DashMap::new(),
            global_environment: DashMap::new(),
            dir_bound: DashMap::new(),
            dir_imported: DashMap::new(),
            dir_expanded: DashMap::new(),
            dir_exported: DashMap::new(),
            dir_resolved: DashMap::new(),
            dir_checked: DashMap::new(),
            dir_materialized: DashMap::new(),
            dir_elaborated: DashMap::new(),
            mir_lowered: DashMap::new(),
            mir_verified: DashMap::new(),
            mir_optimized: DashMap::new(),
            module_query_index: DashMap::new(),
            workspace_query_index: DashMap::new(),
            module_output: DashMap::new(),
            package_output: DashMap::new(),
            module_linted: DashMap::new(),
            package_linted: DashMap::new(),
            workspace_linted: DashMap::new(),
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

    /// Read one data artifact.
    pub fn data(&self, module_id: ModuleId) -> Option<Arc<Data>> {
        self.read_cached(
            &self.data,
            module_id,
            ArtifactKey::data(module_id),
            |version| self.repository.artifact_store().data(version),
        )
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

    /// Read one bound DIR artifact.
    pub fn dir_bound(&self, module_id: ModuleId, profile_id: ProfileId) -> Option<Arc<DirBound>> {
        self.read_cached(
            &self.dir_bound,
            (module_id, profile_id),
            ArtifactKey::dir_bound(module_id, profile_id),
            |version| self.repository.artifact_store().dir_bound(version),
        )
    }

    /// Read one imported DIR artifact.
    pub fn dir_imported(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirImported>> {
        self.read_cached(
            &self.dir_imported,
            (module_id, profile_id),
            ArtifactKey::dir_imported(module_id, profile_id),
            |version| self.repository.artifact_store().dir_imported(version),
        )
    }

    /// Read one expanded DIR artifact.
    pub fn dir_expanded(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirExpanded>> {
        self.read_cached(
            &self.dir_expanded,
            (module_id, profile_id),
            ArtifactKey::dir_expanded(module_id, profile_id),
            |version| self.repository.artifact_store().dir_expanded(version),
        )
    }

    /// Read one exported DIR artifact.
    pub fn dir_exported(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirExported>> {
        self.read_cached(
            &self.dir_exported,
            (module_id, profile_id),
            ArtifactKey::dir_exported(module_id, profile_id),
            |version| self.repository.artifact_store().dir_exported(version),
        )
    }

    /// Read one resolved DIR artifact.
    pub fn dir_resolved(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirResolved>> {
        self.read_cached(
            &self.dir_resolved,
            (module_id, profile_id),
            ArtifactKey::dir_resolved(module_id, profile_id),
            |version| self.repository.artifact_store().dir_resolved(version),
        )
    }

    /// Read one checked DIR artifact.
    pub fn dir_checked(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirChecked>> {
        self.read_cached(
            &self.dir_checked,
            (module_id, profile_id),
            ArtifactKey::dir_checked(module_id, profile_id),
            |version| self.repository.artifact_store().dir_checked(version),
        )
    }

    /// Read one materialized DIR artifact.
    pub fn dir_materialized(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirMaterialized>> {
        self.read_cached(
            &self.dir_materialized,
            (module_id, profile_id),
            ArtifactKey::dir_materialized(module_id, profile_id),
            |version| self.repository.artifact_store().dir_materialized(version),
        )
    }

    /// Read one elaborated DIR artifact.
    pub fn dir_elaborated(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirElaborated>> {
        self.read_cached(
            &self.dir_elaborated,
            (module_id, profile_id),
            ArtifactKey::dir_elaborated(module_id, profile_id),
            |version| self.repository.artifact_store().dir_elaborated(version),
        )
    }

    /// Read one lowered MIR artifact.
    pub fn mir_lowered(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> Option<Arc<MirLowered>> {
        self.read_cached(
            &self.mir_lowered,
            (module_id, profile_id, target_id),
            ArtifactKey::mir_lowered(module_id, profile_id, target_id),
            |version| self.repository.artifact_store().mir_lowered(version),
        )
    }

    /// Read one verified MIR artifact.
    pub fn mir_verified(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> Option<Arc<MirVerified>> {
        self.read_cached(
            &self.mir_verified,
            (module_id, profile_id, target_id),
            ArtifactKey::mir_verified(module_id, profile_id, target_id),
            |version| self.repository.artifact_store().mir_verified(version),
        )
    }

    /// Read one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> Option<Arc<MirOptimized>> {
        self.read_cached(
            &self.mir_optimized,
            (module_id, profile_id, target_id),
            ArtifactKey::mir_optimized(module_id, profile_id, target_id),
            |version| self.repository.artifact_store().mir_optimized(version),
        )
    }

    /// Read one module query index artifact.
    pub fn module_query_index(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<ModuleQueryIndex>> {
        self.read_cached(
            &self.module_query_index,
            (module_id, profile_id),
            ArtifactKey::module_query_index(module_id, profile_id),
            |version| self.repository.artifact_store().module_query_index(version),
        )
    }

    /// Read one workspace query index artifact.
    pub fn workspace_query_index(&self, profile_id: ProfileId) -> Option<Arc<WorkspaceQueryIndex>> {
        self.read_cached(
            &self.workspace_query_index,
            profile_id,
            ArtifactKey::workspace_query_index(profile_id),
            |version| {
                self.repository
                    .artifact_store()
                    .workspace_query_index(version)
            },
        )
    }

    /// Read one generated module output artifact.
    pub fn module_output(
        &self,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Option<Arc<ModuleOutput>> {
        self.read_cached(
            &self.module_output,
            (module_id, target_id),
            ArtifactKey::module_output(module_id, target_id),
            |version| self.repository.artifact_store().module_output(version),
        )
    }

    /// Read one package output artifact.
    pub fn package_output(
        &self,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Option<Arc<PackageOutput>> {
        self.read_cached(
            &self.package_output,
            (package_id, target_id),
            ArtifactKey::package_output(package_id, target_id),
            |version| self.repository.artifact_store().package_output(version),
        )
    }

    /// Read one module lint marker artifact.
    pub fn module_linted(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<ModuleLinted>> {
        self.read_cached(
            &self.module_linted,
            (module_id, profile_id),
            ArtifactKey::module_linted(module_id, profile_id),
            |version| self.repository.artifact_store().module_linted(version),
        )
    }

    /// Read one package lint marker artifact.
    pub fn package_linted(&self, package_id: PackageId) -> Option<Arc<PackageLinted>> {
        self.read_cached(
            &self.package_linted,
            package_id,
            ArtifactKey::package_linted(package_id),
            |version| self.repository.artifact_store().package_linted(version),
        )
    }

    /// Read the workspace lint marker artifact.
    pub fn workspace_linted(&self) -> Option<Arc<WorkspaceLinted>> {
        self.read_cached(
            &self.workspace_linted,
            (),
            ArtifactKey::workspace_linted(),
            |version| self.repository.artifact_store().workspace_linted(version),
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
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store payload before exposing the revision binding
        let key = version.key;

        self.artifact_store()
            .publish(version, payload, dependencies, diagnostics, sidecars);
        self.artifact_versions.insert((revision, key), version);

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

        self.artifact_store()
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

        self.artifact_store()
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
