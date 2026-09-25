use std::sync::Arc;

use tspp_artifact::{ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactOutcome};

use crate::provider::{
    ArtifactAttemptOutcome, ArtifactAttemptRecorder, ArtifactBase, PendingSet, ProviderError,
    ProviderResult,
};
use crate::repository::{Repository, Revision};
use crate::{ArtifactResolution, DependencySetResolution};

/// What one artifact attempt requires in one revision.
#[derive(Debug)]
pub enum ArtifactPlan {
    /// The artifact is already terminal in this revision.
    Done {
        /// The terminal artifact outcome.
        outcome: ArtifactOutcome,
        /// The attempt outcome describing where the result came from.
        attempt: ArtifactAttemptOutcome,
    },
    /// Declared dependencies must become terminal first.
    Park {
        /// The dependency keys that are not yet terminal.
        frontier: Vec<ArtifactKey>,
        /// The complete dependency set to resume with, when collection finished.
        pending_set: Option<PendingSet>,
    },
    /// The provider must run over the exact dependency set.
    Build {
        /// The result available as an incremental provider base.
        base: Option<Arc<ArtifactBase>>,
        /// The exact dependencies feeding this artifact's version.
        dependencies: Arc<[ArtifactDependency]>,
        /// The first terminally failed dependency poisoning this build.
        failed: Option<ArtifactKey>,
    },
}

impl Repository {
    /// Plan one artifact attempt in one revision.
    pub fn plan_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        pending_set: Option<PendingSet>,
        recorder: &ArtifactAttemptRecorder,
        collect: &mut dyn FnMut(Option<Arc<ArtifactBase>>) -> ProviderResult<ArtifactDependencySet>,
    ) -> ProviderResult<ArtifactPlan> {
        // finish immediately when the revision already selects a terminal result
        let resolution = recorder
            .span("resolve", || self.resolve_artifact(revision, &key))
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact {key:?}: {error}"))
            })?;
        match resolution {
            ArtifactResolution::Terminal { outcome, .. } => {
                let attempt = match outcome {
                    ArtifactOutcome::Ok => ArtifactAttemptOutcome::MemoryCached,
                    ArtifactOutcome::Failed(_) => ArtifactAttemptOutcome::Failed,
                };

                return Ok(ArtifactPlan::Done { outcome, attempt });
            }
            ArtifactResolution::Pending { frontier } => {
                return Ok(ArtifactPlan::Park {
                    frontier,
                    pending_set: None,
                });
            }
            ArtifactResolution::Stale => {}
        }

        // select an incremental provider base
        let base = recorder
            .span("select", || self.artifact_base(revision, key))
            .map_err(|error| {
                ProviderError::internal(format!("failed to select artifact base {key:?}: {error}"))
            })?;

        // assemble a saved pending set or a freshly collected one
        let (mut set, mut progress) = match pending_set {
            Some(pending_set) => (pending_set.set, Some(pending_set.resolved)),
            None => {
                let set = recorder
                    .span("collect", || collect(base.clone()))
                    .map_err(|error| {
                        ProviderError::internal(format!(
                            "failed to collect artifact {key:?}: {error}"
                        ))
                    })?;

                (set, None)
            }
        };
        let (dependencies, failed) = loop {
            let resolution = recorder.span("assemble", || {
                self.resolve_dependency_set(revision, key, set, progress.as_deref(), recorder)
            })?;

            match resolution {
                // collect again with the dependencies resolved so far
                DependencySetResolution::Incomplete => {
                    progress = None;
                    set = recorder
                        .span("collect", || collect(base.clone()))
                        .map_err(|error| {
                            ProviderError::internal(format!(
                                "failed to collect artifact {key:?}: {error}"
                            ))
                        })?;
                }
                DependencySetResolution::Pending {
                    frontier,
                    pending_set,
                } => {
                    return Ok(ArtifactPlan::Park {
                        frontier,
                        pending_set,
                    });
                }
                DependencySetResolution::Resolved {
                    dependencies,
                    failed,
                } => break (dependencies, failed),
            }
        };
        let dependencies = Arc::<[ArtifactDependency]>::from(dependencies);
        recorder.record_dependencies(&dependencies);

        // surface a poisoned dependency without probing caches
        if failed.is_some() {
            return Ok(ArtifactPlan::Build {
                base,
                dependencies,
                failed,
            });
        }

        // select a committed payload when the dependency set already produced it
        let reused = recorder
            .span("reuse", || {
                self.select_artifact_version(revision, key, dependencies.clone())
            })
            .map_err(|error| {
                ProviderError::internal(format!(
                    "failed to select reused artifact {key:?}: {error}"
                ))
            })?;
        if reused {
            return Ok(ArtifactPlan::Done {
                outcome: ArtifactOutcome::Ok,
                attempt: ArtifactAttemptOutcome::MemoryCached,
            });
        }

        Ok(ArtifactPlan::Build {
            base,
            dependencies,
            failed: None,
        })
    }
}
