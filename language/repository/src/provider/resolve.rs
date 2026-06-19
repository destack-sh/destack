use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactOutcome, ArtifactVersion,
};

use crate::provider::{ProviderError, ProviderResult};
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
    ) -> ProviderResult<DependencySetResolution> {
        let versions = if set.is_partial {
            self.artifact_bindings(revision, &set.artifacts)?
        } else {
            self.artifact_versions(revision, &set.artifacts)
                .map_err(|error| ProviderError::internal(error.to_string()))?
        };

        // classify and bind each declared artifact dependency
        let mut dependencies = Vec::with_capacity(set.artifacts.len() + set.sources.len());
        let mut pending = Vec::new();
        let mut failed = None;
        for (dependency, version) in set.artifacts.iter().zip(versions) {
            let Some(version) = version else {
                pending.push(*dependency);

                continue;
            };

            match self.artifact_table().outcome(&version) {
                None => pending.push(*dependency),
                Some(ArtifactOutcome::Failed(_)) => {
                    failed.get_or_insert(*dependency);
                    dependencies.push(ArtifactDependency::artifact(version));
                }
                Some(ArtifactOutcome::Ok) => {
                    dependencies.push(ArtifactDependency::artifact(version));
                }
            }
        }

        // fold in primitive source observations
        for source in &set.sources {
            dependencies.push(ArtifactDependency::Source(source.clone()));
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

    /// Return exact stored bindings for dependency keys in this revision.
    fn artifact_bindings(
        &self,
        revision: Revision,
        keys: &[ArtifactKey],
    ) -> ProviderResult<Vec<Option<ArtifactVersion>>> {
        let mut versions = Vec::with_capacity(keys.len());

        // exact bindings are enough while a collector is still partial
        for key in keys {
            let version = self
                .artifact_binding(revision, key)
                .map_err(|error| ProviderError::internal(error.to_string()))?;
            versions.push(version);
        }

        Ok(versions)
    }
}
