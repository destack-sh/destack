use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use tspp_artifact::{
    Artifact, ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection,
    ArtifactProjectionDependency, ArtifactProjectionKey, ArtifactRequirement, ArtifactVersion,
};
use tspp_source::PackageId;

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

    /// Return one package with every package it depends on, the package first.
    pub fn package_closure(&self, package: PackageId) -> Result<Vec<PackageId>, ProviderError> {
        let closure = self
            .repository
            .package_closure(self.revision, package)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(closure)
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

        self.live_owner_version(artifact_key)
    }

    /// Resolve one read beyond the frozen set, recording the observation.
    fn tracked_version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        let Some(context) = self.context else {
            return Err(self.undeclared_read(artifact_key));
        };
        let version = self.live_owner_version(artifact_key)?;
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
        let projection = ArtifactProjection::new(artifact_key, ArtifactProjectionKey::Payload);
        let version = self.projection_version(projection)?;
        let payload = self
            .repository
            .artifact_table()
            .artifact::<A>(&version)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Select a value and track all projections the selector used to determine it.
    pub fn project<A: Artifact, T, P: IntoIterator<Item = ArtifactProjectionKey>>(
        &self,
        key: A::Key,
        select: impl FnOnce(&A) -> (T, P),
    ) -> Result<T, ProviderError> {
        // load the payload selected by the typed artifact key
        let key = A::artifact_key(key);
        let version = self.projection_owner_version(key)?;
        let payload = self
            .repository
            .artifact_table()
            .artifact::<A>(&version)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or(ProviderError::Corrupt { version })?;

        // track every consumed projection before returning the selected value
        let (value, projections) = select(&payload);
        for projection in projections {
            self.projection_version(ArtifactProjection::new(key, projection))?;
        }

        Ok(value)
    }
}
