use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use destack_artifact::{
    Artifact, ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection,
    ArtifactProjectionDependency, ArtifactProjectionFingerprint, ArtifactProjectionKey,
    ArtifactRequirement, ArtifactVersion, InherentExtension, ModuleGraph,
};
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

    /// Read one artifact payload by its typed key.
    pub fn read<A: Artifact>(&self, key: A::Key) -> Result<Arc<A>, ProviderError> {
        let artifact_key = A::artifact_key(key);
        let version = self.version(artifact_key)?;
        let payload = self
            .repository
            .artifact_table()
            .artifact::<A>(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one artifact through its content projection.
    ///
    /// The read depends on the payload's content fingerprint, so rebuilds
    /// that reproduce identical payloads leave dependents current.
    pub fn read_content<A: Artifact>(&self, key: A::Key) -> Result<Arc<A>, ProviderError> {
        self.read_projection(key, ArtifactProjectionKey::Content)
    }

    /// Read one artifact through an exact projection.
    pub fn read_projection<A: Artifact>(
        &self,
        key: A::Key,
        projection: ArtifactProjectionKey,
    ) -> Result<Arc<A>, ProviderError> {
        let artifact_key = A::artifact_key(key);
        let projection = ArtifactProjection::new(artifact_key, projection);
        let version = self.projection_version(projection)?;
        let payload = self
            .repository
            .artifact_table()
            .artifact::<A>(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
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
