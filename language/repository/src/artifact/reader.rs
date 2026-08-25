use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use destack_artifact::{
    Artifact, ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection,
    ArtifactProjectionDependency, ArtifactProjectionKey, ArtifactRequirement, ArtifactVersion,
    DirResolved, Implementation, ModuleGraph,
};
use destack_dir::GlobalSymbolId;
use destack_source::{ModuleId, ProfileId};

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

/// Projection-checked view over resolved component relationships.
#[derive(Debug)]
pub struct ComponentRelationsReader {
    /// The resolved DIR payload owning the relationships.
    resolved: Arc<DirResolved>,
}

impl Debug for ArtifactReader<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
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
            if let Some(version) = self.declared_version(artifact_key) {
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
    fn projection_version(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_some() {
            if let Some(version) = self.declared_version(projection.artifact) {
                return Ok(version);
            }

            // resolve a declared projection against its current owner
            if self
                .find_dependency(ArtifactRequirement::projection(projection))
                .is_some()
            {
                return self.live_owner_version(projection.artifact);
            }

            return self.tracked_projection_version(projection);
        }

        self.version(projection.artifact)
    }

    /// Resolve one projection owner's version at this revision, requiring a ready result.
    fn live_owner_version(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, ProviderError> {
        let version = self
            .repository
            .artifact_version(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;

        self.require_ready_version(artifact_key, version)
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
        let version = self.live_owner_version(projection.artifact)?;
        let fingerprint = self
            .repository
            .artifact_table()
            .projection_fingerprint(&version, &projection)
            .map_err(|error| {
                ProviderError::internal(format!(
                    "failed to fingerprint artifact projection: {error}"
                ))
            })?
            .ok_or(ProviderError::Corrupt { version })?;
        context.observe(ArtifactDependency::Projection(
            ArtifactProjectionDependency::new(projection.artifact, projection.key, fingerprint),
        ));

        Ok(version)
    }

    /// Resolve one projected artifact owner without authorizing a projected value.
    fn projection_owner_version(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_some() {
            if let Some(version) = self.declared_version(artifact_key) {
                return Ok(version);
            }

            // resolve the owner live for a declared projection or a recorded read
            if self.declares_projection_of(artifact_key) || self.context.is_some() {
                return self.live_owner_version(artifact_key);
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

    /// Resolve one complete payload through its declared dependency.
    fn payload_version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        if self.dependencies.is_none() {
            return self.version(artifact_key);
        }

        if let Some(version) = self.declared_version(artifact_key) {
            return Ok(version);
        }

        let payload = ArtifactProjection::new(artifact_key, ArtifactProjectionKey::Payload);
        let is_payload_declared = self
            .find_dependency(ArtifactRequirement::projection(payload))
            .is_some();

        if is_payload_declared {
            self.projection_version(payload)
        } else {
            self.tracked_projection_version(payload)
        }
    }

    /// Return one exact artifact version declared by the frozen dependency set.
    fn declared_version(&self, artifact_key: ArtifactKey) -> Option<ArtifactVersion> {
        let dependency = self.find_dependency(ArtifactRequirement::artifact(artifact_key))?;
        let ArtifactDependency::Artifact(version) = dependency else {
            return None;
        };

        Some(*version)
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

    /// Return whether the declared dependencies hold a projection owned by one artifact.
    fn declares_projection_of(&self, artifact_key: ArtifactKey) -> bool {
        let Some(dependencies) = self.dependencies else {
            return false;
        };
        let index = dependencies.partition_point(|dependency| match dependency {
            ArtifactDependency::Artifact(_) => true,
            ArtifactDependency::Projection(dependency) => {
                dependency.projection().artifact < artifact_key
            }
            ArtifactDependency::Source(_) => false,
        });

        matches!(
            dependencies.get(index),
            Some(ArtifactDependency::Projection(dependency))
                if dependency.projection().artifact == artifact_key
        )
    }

    /// Read one artifact payload by its typed key.
    pub fn read<A: Artifact>(&self, key: A::Key) -> Result<Arc<A>, ProviderError> {
        let artifact_key = A::artifact_key(key);
        let version = self.payload_version(artifact_key)?;
        let payload = self
            .repository
            .artifact_table()
            .artifact::<A>(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read the resolved relationships that determine one module's component edges.
    pub fn component_relations(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<ComponentRelationsReader, ProviderError> {
        let artifact_key = ArtifactKey::dir_resolved(module, profile);
        let projection = ArtifactProjection::new(
            artifact_key,
            ArtifactProjectionKey::DirResolvedComponentRelations,
        );
        let version = self.projection_version(projection)?;
        let resolved = self
            .repository
            .artifact_table()
            .artifact::<DirResolved>(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(ComponentRelationsReader { resolved })
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
            .artifact::<ModuleGraph>(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(ModuleGraphReader {
            artifacts: self,
            key,
            graph,
        })
    }
}

impl ComponentRelationsReader {
    /// Iterate modules that own resolved import or reference targets.
    pub fn target_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.resolved.target_modules()
    }

    /// Iterate extensions with each interface they implement.
    pub fn implementations(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, Option<GlobalSymbolId>, GlobalSymbolId)> + '_ {
        self.resolved.extensions.implementations()
    }
}

impl ModuleGraphReader<'_> {
    /// Return the sorted module universe.
    pub fn modules(&self) -> Result<&[ModuleId], ProviderError> {
        self.require(ArtifactProjectionKey::ModuleGraphModules)?;

        Ok(self.graph.modules())
    }

    /// Return whether one module is part of this graph.
    pub fn contains(&self, module: ModuleId) -> Result<bool, ProviderError> {
        self.require(ArtifactProjectionKey::ModuleGraphModules)?;

        Ok(self.graph.contains(module))
    }

    /// Return outgoing import edges for one module.
    pub fn edges(&self, module: ModuleId) -> Result<Option<Arc<[ModuleId]>>, ProviderError> {
        self.require(ArtifactProjectionKey::ModuleGraphEdges(module))?;

        Ok(self.graph.edges(module))
    }

    /// Return sorted modules reachable from the given roots over import edges.
    pub fn reachable(&self, roots: &[ModuleId]) -> Result<Vec<ModuleId>, ProviderError> {
        let reachable = self.graph.reachable(roots);

        // record the edges the walk observed
        for module in &reachable {
            self.require(ArtifactProjectionKey::ModuleGraphEdges(*module))?;
        }

        Ok(reachable)
    }

    /// Return the implementations of one interface declared across the graph.
    pub fn interface_implementations(
        &self,
        interface: GlobalSymbolId,
    ) -> Result<&[Implementation], ProviderError> {
        self.require(ArtifactProjectionKey::ModuleGraphImplementations(interface))?;

        Ok(self.graph.interface_implementations(interface))
    }

    /// Require one exact module graph projection.
    fn require(&self, projection: ArtifactProjectionKey) -> Result<(), ProviderError> {
        let projection = ArtifactProjection::new(self.key, projection);
        let _version = self.artifacts.projection_version(projection)?;

        Ok(())
    }
}
