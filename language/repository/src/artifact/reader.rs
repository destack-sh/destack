use std::cmp::Ordering;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection,
    ArtifactProjectionDependency, ArtifactProjectionFingerprint, ArtifactProjectionKey,
    ArtifactRequirement, ArtifactTable, ArtifactVersion, Asset, Build, Bundle, Data, DirBound,
    DirChecked, DirDeclared, DirExpanded, DirExported, DirImported, DirMaterialized, DirParsed,
    DirResolved, EnvironmentBound, EnvironmentDeclared, IndexKind, InherentExtension, MirAnalyzed,
    MirElaborated, MirLowered, MirOptimized, MirVerified, ModuleGraph, ModuleIndex, ModuleLinted,
    Object, Product, ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
};
use destack_program::Program;
use destack_source::{ModuleId, PackageId, ProductId, ProfileId, TargetId};

use crate::provider::{ProviderContext, ProviderError};
use crate::repository::{Repository, Revision};

/// Revision-bound read-only view over ready artifacts.
#[derive(Clone)]
pub struct ArtifactReader<'a> {
    /// The repository that binds artifact versions to the revision.
    repository: &'a Repository,
    /// The pinned revision the reader resolves against.
    revision: Revision,
    /// Frozen provider dependencies when reads are restricted.
    dependencies: Option<&'a [ArtifactDependency]>,
    /// The provider context recording execution-time reads.
    context: Option<&'a dyn ProviderContext>,
}

/// Projection-checked read-only view over one component graph.
#[derive(Debug)]
pub struct ModuleGraphReader<'a> {
    /// The artifact reader enforcing provider dependencies.
    artifacts: &'a ArtifactReader<'a>,
    /// The module graph artifact key.
    key: ArtifactKey,
    /// The exact module graph payload.
    graph: Arc<ModuleGraph>,
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
        Self {
            repository,
            revision,
            dependencies: None,
            context: None,
        }
    }

    /// Restrict provider reads to one frozen dependency set.
    pub fn restrict(mut self, dependencies: &'a [ArtifactDependency]) -> Self {
        self.dependencies = Some(dependencies);

        self
    }

    /// Record reads beyond the frozen set into one provider context.
    pub fn with_context(mut self, context: &'a dyn ProviderContext) -> Self {
        self.context = Some(context);

        self
    }

    /// Resolve one artifact to its exact ready version.
    pub fn version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_some() {
            let version = self
                .find_dependency(ArtifactRequirement::artifact(artifact_key))
                .and_then(|dependency| match dependency {
                    ArtifactDependency::Artifact(version) => Some(*version),
                    ArtifactDependency::Projection(_) | ArtifactDependency::Source(_) => None,
                });
            if let Some(version) = version {
                return Ok(version);
            }

            return self.tracked_version(artifact_key);
        }

        let version = self
            .repository
            .artifact_version(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;

        self.require_ready_version(artifact_key, version)
    }

    /// Resolve one read beyond the frozen set, recording the observation.
    ///
    /// A read the collect pass did not declare blocks until ready; the worker
    /// parks the attempt on the missing artifact and re-runs the provider.
    fn tracked_version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        let Some(context) = self.context else {
            return Err(self.undeclared_read(artifact_key));
        };
        let version = self
            .repository
            .artifact_version(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;
        let version = self.require_ready_version(artifact_key, version)?;
        context.observe(ArtifactDependency::Artifact(version));

        Ok(version)
    }

    /// Resolve one declared artifact projection to its exact owner version.
    pub fn projection_version(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_some() {
            let artifact = self
                .find_dependency(ArtifactRequirement::artifact(projection.artifact))
                .and_then(|dependency| match dependency {
                    ArtifactDependency::Artifact(version) => Some(*version),
                    ArtifactDependency::Projection(_) | ArtifactDependency::Source(_) => None,
                });
            let version = artifact.or_else(|| {
                self.find_dependency(ArtifactRequirement::projection(projection))
                    .and_then(|dependency| match dependency {
                        ArtifactDependency::Projection(dependency) => Some(dependency.version()),
                        ArtifactDependency::Artifact(_) | ArtifactDependency::Source(_) => None,
                    })
            });
            if let Some(version) = version {
                return Ok(version);
            }

            return self.tracked_projection_version(projection);
        }

        self.version(projection.artifact)
    }

    /// Resolve one projection read beyond the frozen set, recording the observation.
    ///
    /// A read the collect pass did not declare blocks until ready; the worker
    /// parks the attempt on the missing artifact and re-runs the provider.
    fn tracked_projection_version(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactVersion, ProviderError> {
        let Some(context) = self.context else {
            return Err(self.undeclared_read(projection.artifact));
        };
        let version = self
            .repository
            .artifact_version(self.revision, &projection.artifact)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;
        let version = self.require_ready_version(projection.artifact, version)?;
        let fingerprint = self
            .repository
            .artifact_table()
            .projection_fingerprint(&version, &projection)
            .ok_or(ProviderError::Corrupt { version })?;
        context.observe(ArtifactDependency::Projection(
            ArtifactProjectionDependency::new(version, projection.key, fingerprint),
        ));

        Ok(version)
    }

    /// Read one declared artifact projection fingerprint.
    pub fn projection_fingerprint(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactProjectionFingerprint, ProviderError> {
        let version = self.projection_version(projection)?;
        let fingerprint = self
            .repository
            .artifact_table()
            .projection_fingerprint(&version, &projection)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(fingerprint)
    }

    /// Resolve one projected artifact owner without authorizing a projected value.
    fn projection_owner_version(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_some() {
            let artifact = self
                .find_dependency(ArtifactRequirement::artifact(artifact_key))
                .and_then(|dependency| match dependency {
                    ArtifactDependency::Artifact(version) => Some(*version),
                    ArtifactDependency::Projection(_) | ArtifactDependency::Source(_) => None,
                });
            let version = artifact.or_else(|| self.find_projection_owner(artifact_key));
            if let Some(version) = version {
                return Ok(version);
            }

            // resolve the owner version live for recorded reads
            if self.context.is_some() {
                let version = self
                    .repository
                    .artifact_version(self.revision, &artifact_key)
                    .map_err(|error| {
                        ProviderError::internal(format!(
                            "failed to resolve artifact version: {error}"
                        ))
                    })?;

                return self.require_ready_version(artifact_key, version);
            }

            return Err(self.undeclared_read(artifact_key));
        }

        self.version(artifact_key)
    }

    /// Require one resolved artifact version to carry a ready result.
    fn require_ready_version(
        &self,
        artifact_key: ArtifactKey,
        version: Option<ArtifactVersion>,
    ) -> Result<ArtifactVersion, ProviderError> {
        let Some(version) = version else {
            // report the blocked key so collection requires it
            if let Some(context) = self.context {
                context.record_blocked(artifact_key);
            }

            return Err(ProviderError::blocked(artifact_key));
        };
        match self.repository.artifact_table().outcome(&version) {
            Some(ArtifactOutcome::Ok) => Ok(version),
            Some(ArtifactOutcome::Failed(_)) => {
                Err(ProviderError::RequirementFailed { key: artifact_key })
            }
            None => Err(ProviderError::Corrupt { version }),
        }
    }

    /// Return the error for one read outside the frozen dependency set.
    fn undeclared_read(&self, artifact_key: ArtifactKey) -> ProviderError {
        ProviderError::internal(format!(
            "provider read undeclared artifact {artifact_key:?}"
        ))
    }

    /// Find one exact declared artifact requirement.
    fn find_dependency(&self, requirement: ArtifactRequirement) -> Option<&ArtifactDependency> {
        let dependencies = self.dependencies?;
        let index = dependencies
            .binary_search_by(|dependency| match dependency.requirement() {
                Some(dependency) => dependency.cmp(&requirement),
                None => Ordering::Greater,
            })
            .ok()?;

        dependencies.get(index)
    }

    /// Find any declared projection owned by one artifact.
    fn find_projection_owner(&self, artifact_key: ArtifactKey) -> Option<ArtifactVersion> {
        let dependencies = self.dependencies?;
        let index = dependencies.partition_point(|dependency| match dependency {
            ArtifactDependency::Artifact(_) => true,
            ArtifactDependency::Projection(dependency) => {
                dependency.projection().artifact < artifact_key
            }
            ArtifactDependency::Source(_) => false,
        });
        let dependency = dependencies.get(index)?;
        let ArtifactDependency::Projection(dependency) = dependency else {
            return None;
        };
        if dependency.projection().artifact != artifact_key {
            return None;
        }

        Some(dependency.version())
    }

    /// Read one collected typed artifact payload.
    fn read<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactTable, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        let version = self.version(artifact_key)?;
        let payload = get(self.repository.artifact_table(), &version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one artifact through its content projection.
    ///
    /// The read depends on the payload's content fingerprint, so rebuilds
    /// that reproduce identical payloads leave dependents current.
    fn read_content<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactTable, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        let projection = ArtifactProjection::new(artifact_key, ArtifactProjectionKey::Content);
        let version = self.projection_version(projection)?;
        let payload = get(self.repository.artifact_table(), &version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one declared DIR artifact through its declared projection.
    pub fn dir_declared_projected(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ProviderError> {
        let key = ArtifactKey::dir_declared(module, profile);
        let projection = ArtifactProjection::new(key, ArtifactProjectionKey::Declared);
        let version = self.projection_version(projection)?;
        let payload = self
            .repository
            .artifact_table()
            .dir_declared(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one bound DIR artifact through its content projection.
    pub fn dir_bound_content(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        self.read_content(
            ArtifactKey::dir_bound(module, profile),
            ArtifactTable::dir_bound,
        )
    }

    /// Read one expanded DIR artifact through its content projection.
    pub fn dir_expanded_content(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        self.read_content(
            ArtifactKey::dir_expanded(module, profile),
            ArtifactTable::dir_expanded,
        )
    }

    /// Read one resolved DIR artifact through its content projection.
    pub fn dir_resolved_content(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        self.read_content(
            ArtifactKey::dir_resolved(module, profile),
            ArtifactTable::dir_resolved,
        )
    }

    /// Read one exported DIR artifact through its content projection.
    pub fn dir_exported_content(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        self.read_content(
            ArtifactKey::dir_exported(module, profile),
            ArtifactTable::dir_exported,
        )
    }

    /// Read one parsed DIR artifact.
    pub fn dir_parsed(&self, module: ModuleId) -> Result<Arc<DirParsed>, ProviderError> {
        self.read(ArtifactKey::dir_parsed(module), ArtifactTable::dir_parsed)
    }

    /// Read one data artifact.
    pub fn data(&self, module: ModuleId) -> Result<Arc<Data>, ProviderError> {
        self.read(ArtifactKey::data(module), ArtifactTable::data)
    }

    /// Read one bound environment artifact.
    pub fn environment_bound(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<EnvironmentBound>, ProviderError> {
        self.read(
            ArtifactKey::environment_bound(profile),
            ArtifactTable::environment_bound,
        )
    }

    /// Read one bound environment artifact through its content projection.
    pub fn environment_bound_content(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<EnvironmentBound>, ProviderError> {
        self.read_content(
            ArtifactKey::environment_bound(profile),
            ArtifactTable::environment_bound,
        )
    }

    /// Read one declared environment artifact.
    pub fn environment_declared(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<EnvironmentDeclared>, ProviderError> {
        self.read(
            ArtifactKey::environment_declared(profile),
            ArtifactTable::environment_declared,
        )
    }

    /// Read one module graph artifact.
    pub fn module_graph(&self, profile: ProfileId) -> Result<Arc<ModuleGraph>, ProviderError> {
        self.read(
            ArtifactKey::module_graph(profile),
            ArtifactTable::module_graph,
        )
    }

    /// Read one module graph through exact projected values.
    pub fn module_graph_reader(
        &'a self,
        profile: ProfileId,
    ) -> Result<ModuleGraphReader<'a>, ProviderError> {
        let key = ArtifactKey::module_graph(profile);
        let version = self.projection_owner_version(key)?;
        let graph = self
            .repository
            .artifact_table()
            .module_graph(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(ModuleGraphReader {
            artifacts: self,
            key,
            graph,
        })
    }

    /// Read one bound DIR artifact.
    pub fn dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        self.read(
            ArtifactKey::dir_bound(module, profile),
            ArtifactTable::dir_bound,
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
            ArtifactTable::dir_imported,
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
            ArtifactTable::dir_expanded,
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
            ArtifactTable::dir_exported,
        )
    }

    /// Read one exported DIR artifact through an exact projection.
    pub fn dir_exported_projection(
        &self,
        module: ModuleId,
        profile: ProfileId,
        projection: ArtifactProjectionKey,
    ) -> Result<Arc<DirExported>, ProviderError> {
        let key = ArtifactKey::dir_exported(module, profile);
        let projection = ArtifactProjection::new(key, projection);
        let version = self.projection_version(projection)?;
        let exported = self
            .repository
            .artifact_table()
            .dir_exported(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(exported)
    }

    /// Read one resolved DIR artifact.
    pub fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        self.read(
            ArtifactKey::dir_resolved(module, profile),
            ArtifactTable::dir_resolved,
        )
    }

    /// Read one resolved DIR artifact through an exact projection.
    pub fn dir_resolved_projection(
        &self,
        module: ModuleId,
        profile: ProfileId,
        projection: ArtifactProjectionKey,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        let key = ArtifactKey::dir_resolved(module, profile);
        let projection = ArtifactProjection::new(key, projection);
        let version = self.projection_version(projection)?;
        let resolved = self
            .repository
            .artifact_table()
            .dir_resolved(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(resolved)
    }

    /// Read one declared DIR artifact.
    pub fn dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ProviderError> {
        self.read(
            ArtifactKey::dir_declared(module, profile),
            ArtifactTable::dir_declared,
        )
    }

    /// Read one checked DIR artifact.
    pub fn dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirChecked>, ProviderError> {
        self.read(
            ArtifactKey::dir_checked(module, profile),
            ArtifactTable::dir_checked,
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
            ArtifactTable::dir_materialized,
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
            ArtifactTable::mir_lowered,
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
            ArtifactTable::mir_verified,
        )
    }

    /// Read one elaborated MIR artifact.
    pub fn mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirElaborated>, ProviderError> {
        self.read(
            ArtifactKey::mir_elaborated(module, profile, target),
            ArtifactTable::mir_elaborated,
        )
    }

    /// Read one analyzed MIR link summary.
    pub fn mir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirAnalyzed>, ProviderError> {
        self.read(
            ArtifactKey::mir_analyzed(module, profile, target),
            ArtifactTable::mir_analyzed,
        )
    }

    /// Read one whole-program analysis artifact.
    pub fn program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ProgramAnalysis>, ProviderError> {
        self.read(
            ArtifactKey::program_analysis(profile, target),
            ArtifactTable::program_analysis,
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
            ArtifactTable::mir_optimized,
        )
    }

    /// Read one module index artifact.
    pub fn module_index(
        &self,
        module: ModuleId,
        profile: ProfileId,
        kind: IndexKind,
    ) -> Result<Arc<ModuleIndex>, ProviderError> {
        self.read(
            ArtifactKey::module_index(module, profile, kind),
            ArtifactTable::module_index,
        )
    }

    /// Read one program index artifact.
    pub fn program_index(
        &self,
        profile: ProfileId,
        kind: IndexKind,
    ) -> Result<Arc<ProgramIndex>, ProviderError> {
        self.read(
            ArtifactKey::program_index(profile, kind),
            ArtifactTable::program_index,
        )
    }

    /// Read one structured script artifact.
    pub fn script(&self, module: ModuleId, target: TargetId) -> Result<Arc<Script>, ProviderError> {
        self.read(ArtifactKey::script(module, target), ArtifactTable::script)
    }

    /// Read one compiled-code object artifact.
    pub fn object(&self, module: ModuleId, target: TargetId) -> Result<Arc<Object>, ProviderError> {
        self.read(ArtifactKey::object(module, target), ArtifactTable::object)
    }

    /// Read one asset artifact.
    pub fn asset(&self, module: ModuleId, target: TargetId) -> Result<Arc<Asset>, ProviderError> {
        self.read(ArtifactKey::asset(module, target), ArtifactTable::asset)
    }

    /// Read one build payload.
    pub fn build(&self, target: TargetId) -> Result<Arc<Build>, ProviderError> {
        self.read(ArtifactKey::build(target), ArtifactTable::build)
    }

    /// Read one bundle artifact.
    pub fn bundle(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<Bundle>, ProviderError> {
        self.read(ArtifactKey::bundle(package, target), ArtifactTable::bundle)
    }

    /// Read one executable program artifact.
    pub fn program(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<Program>, ProviderError> {
        self.read(
            ArtifactKey::program(package, target),
            ArtifactTable::program,
        )
    }

    /// Read one product artifact.
    pub fn product(
        &self,
        package: PackageId,
        product: ProductId,
    ) -> Result<Arc<Product>, ProviderError> {
        self.read(
            ArtifactKey::product(package, product),
            ArtifactTable::product,
        )
    }

    /// Read one module lint marker artifact.
    pub fn module_linted(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ModuleLinted>, ProviderError> {
        self.read(
            ArtifactKey::module_linted(module, profile, target),
            ArtifactTable::module_linted,
        )
    }

    /// Read one program lint marker artifact.
    pub fn program_linted(
        &self,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ProgramLinted>, ProviderError> {
        self.read(
            ArtifactKey::program_linted(profile, target),
            ArtifactTable::program_linted,
        )
    }
}

impl ModuleGraphReader<'_> {
    /// Return the sorted module universe.
    pub fn modules(&self) -> Result<&[ModuleId], ProviderError> {
        self.require(ArtifactProjectionKey::Modules)?;

        Ok(self.graph.modules())
    }

    /// Return whether one module is part of this graph.
    pub fn contains(&self, module: ModuleId) -> Result<bool, ProviderError> {
        self.require(ArtifactProjectionKey::Modules)?;

        Ok(self.graph.contains(module))
    }

    /// Return outgoing import edges for one module.
    pub fn edges(&self, module: ModuleId) -> Result<Option<Arc<[ModuleId]>>, ProviderError> {
        self.require(ArtifactProjectionKey::ModuleEdges(module))?;

        Ok(self.graph.edges(module))
    }

    /// Return sorted modules reachable from the given roots over import edges.
    pub fn reachable(&self, roots: &[ModuleId]) -> Result<Vec<ModuleId>, ProviderError> {
        let reachable = self.graph.reachable(roots);

        // record the edges the walk observed
        for module in &reachable {
            self.require(ArtifactProjectionKey::ModuleEdges(*module))?;
        }

        Ok(reachable)
    }

    /// Return the inherent extensions declared outside their target's module.
    pub fn cross_module_extensions(
        &self,
    ) -> Result<impl Iterator<Item = &InherentExtension>, ProviderError> {
        self.require(ArtifactProjectionKey::InherentExtensions)?;

        Ok(self.graph.cross_module_extensions())
    }

    /// Require one exact module graph projection.
    fn require(&self, projection: ArtifactProjectionKey) -> Result<(), ProviderError> {
        let projection = ArtifactProjection::new(self.key, projection);
        let _version = self.artifacts.projection_version(projection)?;

        Ok(())
    }
}
