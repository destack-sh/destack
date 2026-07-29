use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactOutcome, ArtifactRequirement,
    ArtifactVersion,
};

use crate::provider::{ArtifactBase, ProviderError, ProviderResult};
use crate::repository::{Repository, Revision};

/// The repository resolution of one collected artifact dependency set.
#[derive(Debug)]
pub enum DependencySetResolution {
    /// Collection stopped before it named a complete set and nothing is waiting.
    Incomplete,
    /// The dependency set names artifacts that are not yet terminal.
    Pending {
        /// The declared dependency keys that are not yet terminal.
        frontier: Vec<ArtifactKey>,
        /// The complete dependency set, present when collection did not stop early.
        pending_set: Option<ArtifactDependencySet>,
    },
    /// The dependency set resolved into exact dependencies.
    Resolved {
        /// The exact dependencies feeding this artifact's version.
        dependencies: Vec<ArtifactDependency>,
        /// The first terminally failed dependency, when one poisons the build.
        failed: Option<ArtifactKey>,
    },
}

impl Repository {
    /// Resolve one collected dependency set into exact dependencies when possible.
    pub fn resolve_dependency_set(
        &self,
        revision: Revision,
        set: ArtifactDependencySet,
        base: Option<&ArtifactBase>,
    ) -> ProviderResult<DependencySetResolution> {
        let base = base.filter(|base| set.matches(&base.dependencies));
        let artifact_keys = set
            .requirements
            .iter()
            .enumerate()
            .filter_map(|(dependency, requirement)| {
                let is_dirty = base.is_none_or(|base| base.is_dependency_dirty(dependency));
                is_dirty.then_some(requirement.artifact_key())
            })
            .collect::<Vec<_>>();
        let versions = self
            .artifact_versions(revision, &artifact_keys)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let mut versions = versions.into_iter();

        // classify each declared artifact requirement
        let capacity = set.requirements.len() + set.sources.len();
        let mut dependencies = Vec::with_capacity(capacity);
        let mut pending = Vec::new();
        let mut failed = None;
        for (dependency, requirement) in set.requirements.iter().enumerate() {
            let previous = base
                .filter(|base| !base.is_dependency_dirty(dependency))
                .map(|base| base.dependencies[dependency].clone());

            if let Some(previous) = previous {
                dependencies.push(previous);
            } else {
                let version = versions.next().ok_or_else(|| {
                    ProviderError::internal("resolved dependency versions are incomplete")
                })?;
                self.resolve_requirement(
                    *requirement,
                    version,
                    &mut dependencies,
                    &mut pending,
                    &mut failed,
                )?;
            }
        }

        // fold in primitive source observations
        for source in &set.sources {
            dependencies.push(ArtifactDependency::Source(*source));
        }

        // resolve immediately when a dependency already failed
        if let Some(failed) = failed {
            return Ok(DependencySetResolution::Resolved {
                dependencies,
                failed: Some(failed),
            });
        }

        // park until the frontier becomes terminal, carrying complete dependency sets
        if !pending.is_empty() {
            let pending_set = (!set.is_partial).then_some(set);

            return Ok(DependencySetResolution::Pending {
                frontier: pending,
                pending_set,
            });
        }

        if set.is_partial {
            return Ok(DependencySetResolution::Incomplete);
        }

        Ok(DependencySetResolution::Resolved {
            dependencies,
            failed: None,
        })
    }

    /// Resolve one collected artifact requirement into an exact dependency.
    fn resolve_requirement(
        &self,
        requirement: ArtifactRequirement,
        version: Option<ArtifactVersion>,
        dependencies: &mut Vec<ArtifactDependency>,
        pending: &mut Vec<ArtifactKey>,
        failed: &mut Option<ArtifactKey>,
    ) -> ProviderResult<()> {
        let artifact_key = requirement.artifact_key();
        let Some(version) = version else {
            pending.push(artifact_key);

            return Ok(());
        };

        match self.artifact_table().outcome(&version) {
            None => {
                return Err(ProviderError::internal(format!(
                    "resolved artifact version is missing: {version:?}"
                ))
                .into());
            }
            Some(ArtifactOutcome::Failed(_)) => {
                failed.get_or_insert(artifact_key);
                dependencies.push(ArtifactDependency::artifact(version));
            }
            Some(ArtifactOutcome::Ok) => {
                self.resolve_completed_requirement(requirement, version, dependencies)?;
            }
        }

        Ok(())
    }

    /// Resolve one completed artifact requirement into an exact dependency.
    fn resolve_completed_requirement(
        &self,
        requirement: ArtifactRequirement,
        version: ArtifactVersion,
        dependencies: &mut Vec<ArtifactDependency>,
    ) -> ProviderResult<()> {
        match requirement {
            ArtifactRequirement::Artifact(_) => {
                dependencies.push(ArtifactDependency::artifact(version));
            }
            ArtifactRequirement::Projection(projection) => {
                let fingerprint = self
                    .artifact_table()
                    .projection_fingerprint(&version, &projection)
                    .ok_or_else(|| {
                        ProviderError::internal(format!(
                            "artifact projection does not match payload: {projection:?}"
                        ))
                    })?;
                dependencies.push(ArtifactDependency::projection(
                    version,
                    projection.key,
                    fingerprint,
                ));
            }
        }

        Ok(())
    }
}
