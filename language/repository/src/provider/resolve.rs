use tspp_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactOutcome, ArtifactRequirement,
    ArtifactVersion,
};

use crate::ArtifactResolution;
use crate::provider::{ArtifactAttemptRecorder, ProviderError, ProviderResult};
use crate::repository::{Repository, Revision};

/// One parked attempt's dependency set and its resolved slots.
#[derive(Debug)]
pub struct PendingSet {
    /// The normalized dependency set carried across the park.
    pub set: ArtifactDependencySet,
    /// The exact dependency resolved per requirement slot, none while pending.
    pub resolved: Vec<Option<ArtifactDependency>>,
}

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
        pending_set: Option<PendingSet>,
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
        key: ArtifactKey,
        mut set: ArtifactDependencySet,
        progress: Option<&[Option<ArtifactDependency>]>,
        recorder: &ArtifactAttemptRecorder,
    ) -> ProviderResult<DependencySetResolution> {
        let artifact_requirements = set.requirements.len() as u64;
        let source_dependencies = set.sources.len() as u64;

        // parked sets are already normalized and slot aligned with progress
        if progress.is_none() {
            recorder.breakdown("assemble.normalize", || set.normalize());
        }
        let resolved_slot = |dependency: usize| {
            progress.and_then(|progress| progress.get(dependency).cloned().flatten())
        };
        let artifact_keys = set
            .requirements
            .iter()
            .enumerate()
            .filter_map(|(dependency, requirement)| {
                let is_unresolved = resolved_slot(dependency).is_none();
                is_unresolved.then_some(requirement.artifact_key())
            })
            .collect::<Vec<_>>();
        let unresolved_requirements = artifact_keys.len() as u64;
        recorder.record_counters(&[
            ("assemble.requirements", artifact_requirements),
            ("assemble.sources", source_dependencies),
            ("assemble.unresolved", unresolved_requirements),
        ]);
        let resolutions = recorder.breakdown("assemble.resolve", || {
            self.resolve_artifacts(revision, &artifact_keys)
                .map_err(|error| ProviderError::internal(error.to_string()))
        })?;
        let mut resolutions = resolutions.into_iter();

        // classify each declared artifact requirement
        let (dependencies, slots, pending, failed) =
            recorder.breakdown("assemble.classify", || -> ProviderResult<_> {
                let capacity = set.requirements.len() + set.sources.len();
                let mut dependencies = Vec::with_capacity(capacity);
                let mut slots = Vec::with_capacity(set.requirements.len());
                let mut pending = Vec::new();
                let mut failed = None;
                for (dependency, requirement) in set.requirements.iter().enumerate() {
                    // carry dependencies resolved before this attempt parked
                    let carried = resolved_slot(dependency);

                    if let Some(carried) = carried {
                        slots.push(Some(dependencies.len() as u32));
                        dependencies.push(carried);
                    } else {
                        let resolution = resolutions.next().ok_or_else(|| {
                            ProviderError::internal("resolved dependencies are incomplete")
                        })?;
                        let before = dependencies.len();
                        self.resolve_requirement(
                            *requirement,
                            resolution,
                            key.requires_clean_dependencies(),
                            &mut dependencies,
                            &mut pending,
                            &mut failed,
                        )?;
                        let is_resolved = dependencies.len() > before;
                        slots.push(is_resolved.then_some(before as u32));
                    }
                }

                // fold in primitive source observations
                for source in &set.sources {
                    dependencies.push(ArtifactDependency::Source(*source));
                }

                Ok((dependencies, slots, pending, failed))
            })?;

        // resolve immediately when a dependency already failed
        if let Some(failed) = failed {
            return Ok(DependencySetResolution::Resolved {
                dependencies,
                failed: Some(failed),
            });
        }

        // park until the frontier becomes terminal, carrying complete
        //  dependency sets and the slots resolved so far
        if !pending.is_empty() {
            let pending_set = (!set.is_partial).then(|| {
                let resolved = slots
                    .iter()
                    .map(|slot| slot.map(|index| dependencies[index as usize].clone()))
                    .collect();

                PendingSet { set, resolved }
            });

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
        resolution: ArtifactResolution,
        requires_clean: bool,
        dependencies: &mut Vec<ArtifactDependency>,
        pending: &mut Vec<ArtifactKey>,
        failed: &mut Option<ArtifactKey>,
    ) -> ProviderResult<()> {
        let artifact_key = requirement.artifact_key();
        match resolution {
            ArtifactResolution::Terminal {
                version,
                outcome: ArtifactOutcome::Failed(_),
            } => {
                failed.get_or_insert(artifact_key);
                dependencies.push(ArtifactDependency::artifact(version));
            }
            // poison a clean-inputs build on a dependency that reported errors
            ArtifactResolution::Terminal {
                version,
                outcome: ArtifactOutcome::Ok,
            } if requires_clean && self.artifact_has_errors(&version)? => {
                failed.get_or_insert(artifact_key);
                dependencies.push(ArtifactDependency::artifact(version));
            }
            ArtifactResolution::Terminal {
                version,
                outcome: ArtifactOutcome::Ok,
            } => {
                self.resolve_completed_requirement(requirement, version, dependencies)?;
            }
            ArtifactResolution::Pending {
                frontier: dependency_frontier,
            } => {
                pending.extend(dependency_frontier);
            }
            ArtifactResolution::Stale => {
                pending.push(artifact_key);
            }
        }

        Ok(())
    }

    /// Return whether one terminal artifact version reported error diagnostics.
    fn artifact_has_errors(&self, version: &ArtifactVersion) -> ProviderResult<bool> {
        self.artifact_table().has_errors(version).ok_or_else(|| {
            ProviderError::internal(format!(
                "terminal artifact has no result entry: {version:?}"
            ))
            .into()
        })
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
                    .map_err(|error| ProviderError::internal(error.to_string()))?
                    .ok_or_else(|| {
                        ProviderError::internal(format!(
                            "artifact projection does not match payload: {projection:?}"
                        ))
                    })?;
                dependencies.push(ArtifactDependency::projection(
                    projection.artifact,
                    projection.key,
                    fingerprint,
                ));
            }
        }

        Ok(())
    }
}
