use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactProvider, ArtifactSidecar, ArtifactVersion,
};
use destack_repository::{
    ArtifactAttemptOutcome, ArtifactResolution, DependencySetResolution, ProviderError,
    ProviderResult, Revision,
};
use destack_source::DiagnosticCollection;

use super::attempt::ProviderAttempt;
use super::run::{ArtifactRunId, ArtifactRunState};
use super::scheduler::Scheduler;
use super::task::Task;
use crate::{SessionError, SessionEvent, SessionState};

/// One worker in the session artifact executor.
#[derive(Debug, Clone)]
pub(super) struct Worker {
    /// The index of this worker in the session pool.
    pub(super) index: usize,
    /// Shared session state for provider execution.
    pub(super) state: Arc<SessionState>,
    /// Shared scheduler for artifact work.
    pub(super) scheduler: Arc<Scheduler>,
}

impl SessionState {
    /// Collect the dependency set for the system that owns one key.
    fn collect_dependencies(
        &self,
        revision: Revision,
        key: ArtifactKey,
        base: Option<Arc<destack_repository::ArtifactBase>>,
    ) -> ProviderResult<ArtifactDependencySet> {
        let attempt = ProviderAttempt::new(self.repository(), revision, key).with_base(base);

        match key.provider() {
            ArtifactProvider::Loader => self
                .collect_loader(&attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.compiler().collect(&attempt),
            ArtifactProvider::Linter => self.linter().collect(&attempt),
            ArtifactProvider::Index => self.indexer().collect(&attempt),
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Worker {
    /// Drive the shared scheduler until the executor shuts down.
    pub(super) fn run(&self) {
        loop {
            let Some((run, task, pending_set)) = self.scheduler.claim() else {
                return;
            };

            // finish tasks whose revision binding is already resolved
            match self.state.artifact_outcome(task) {
                Ok(Some(_)) => {
                    self.scheduler.mark_done(task);
                }
                Ok(None) => {
                    if let Err(error) = self.provide_task(run.as_ref(), task, pending_set) {
                        self.scheduler.abort(task, error);
                    }
                }
                Err(error) => {
                    self.scheduler.abort(task, error);
                }
            }

            // finish a cancelled trace after its last active attempt
            if run.is_abandoned() && !self.scheduler.is_run_executing(run.id()) {
                run.finish(&self.scheduler, &self.state);
            }
        }
    }

    /// Provide one claimed task.
    pub(super) fn provide_task(
        &self,
        run: &ArtifactRunState,
        task: Task,
        pending_set: Option<ArtifactDependencySet>,
    ) -> Result<(), SessionError> {
        let recorder = Arc::new(run.trace().begin(task.key, self.index));

        // expose the session task through events
        self.state.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        let repository = self.state.repository();

        // resolve one stored artifact before collecting new dependencies
        let resolution = recorder.span("resolve", || {
            repository.resolve_artifact(task.revision, &task.key)
        });
        match resolution {
            Ok(ArtifactResolution::Terminal { outcome, .. }) => {
                match outcome {
                    ArtifactOutcome::Ok => {
                        recorder.finish(ArtifactAttemptOutcome::MemoryCached);
                        self.finish_ready(run.id(), task);
                    }
                    ArtifactOutcome::Failed(_) => {
                        recorder.finish(ArtifactAttemptOutcome::Failed);
                        self.finish_failed(run.id(), task);
                    }
                }

                return Ok(());
            }
            Ok(ArtifactResolution::Pending { frontier }) => {
                recorder.record_counter("frontier", frontier.len() as u64);
                let frontier = frontier
                    .into_iter()
                    .map(|key| Task::new(task.revision, key))
                    .collect();
                let result =
                    recorder.span("scheduler", || self.scheduler.wait_on(task, frontier, None));
                let outcome = if result.is_ok() {
                    ArtifactAttemptOutcome::Parked
                } else {
                    ArtifactAttemptOutcome::Failed
                };
                recorder.finish(outcome);

                return result;
            }
            Ok(ArtifactResolution::Stale) => {}
            Err(error) => {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to resolve artifact {:?}: {error}", task.key),
                });
            }
        }

        let base = recorder.span("base", || repository.artifact_base(task.revision, task.key));
        let base = match base {
            Ok(base) => base,
            Err(error) => {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to select artifact base {:?}: {error}", task.key),
                });
            }
        };

        // resolve a saved pending set or collect a fresh one
        let mut collected = if let Some(pending_set) = pending_set {
            Ok(pending_set)
        } else {
            recorder.span("collect", || {
                self.state
                    .collect_dependencies(task.revision, task.key, base.clone())
            })
        };
        let (dependencies, failed) = loop {
            let dependency_set = match collected {
                Ok(dependency_set) => dependency_set,
                Err(error) => {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(SessionError::Internal {
                        detail: format!("failed to collect artifact {:?}: {error}", task.key),
                    });
                }
            };
            let resolution = recorder.span("dependencies", || {
                repository.resolve_dependency_set(
                    task.revision,
                    dependency_set,
                    base.as_deref(),
                    recorder.as_ref(),
                )
            });
            let resolution = match resolution {
                Ok(resolution) => resolution,
                Err(error) => {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(SessionError::Internal {
                        detail: format!(
                            "failed to resolve artifact dependencies {:?}: {error}",
                            task.key
                        ),
                    });
                }
            };

            match resolution {
                DependencySetResolution::Incomplete => {
                    collected = recorder.span("collect", || {
                        self.state
                            .collect_dependencies(task.revision, task.key, base.clone())
                    });
                }
                DependencySetResolution::Pending {
                    frontier,
                    pending_set,
                } => {
                    recorder.record_counter("frontier", frontier.len() as u64);
                    let frontier = frontier
                        .into_iter()
                        .map(|key| Task::new(task.revision, key))
                        .collect();
                    let result = recorder.span("scheduler", || {
                        self.scheduler.wait_on(task, frontier, pending_set)
                    });
                    let outcome = if result.is_ok() {
                        ArtifactAttemptOutcome::Parked
                    } else {
                        ArtifactAttemptOutcome::Failed
                    };
                    recorder.finish(outcome);

                    return result;
                }
                DependencySetResolution::Resolved {
                    dependencies,
                    failed,
                } => break (dependencies, failed),
            }
        };
        let dependencies = Arc::<[ArtifactDependency]>::from(dependencies);
        recorder.record_dependencies(&dependencies);

        // identify the reusable result from its dependency observations
        let version = recorder.span("version", || {
            ArtifactVersion::new(
                task.key,
                repository.build_fingerprint(),
                dependencies.iter().cloned(),
            )
        });

        // fail this artifact immediately on a poisoned dependency
        if let Some(failed_dependency) = failed {
            let failure = ArtifactFailure::requirement(failed_dependency);
            let diagnostics = DiagnosticCollection::new();
            let sidecars = Vec::new();
            let result = recorder.span("complete", || {
                self.fail(
                    run.id(),
                    task,
                    version,
                    dependencies,
                    diagnostics,
                    sidecars,
                    failure,
                )
            });
            recorder.finish(ArtifactAttemptOutcome::Failed);

            return result;
        }

        // bind a committed payload when the dependency set already produced it
        let reused = recorder.span("memory_cache", || {
            repository.bind_artifact_version(task.revision, version, Arc::clone(&dependencies))
        });
        let reused = match reused {
            Ok(reused) => reused,
            Err(error) => {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to bind reused artifact {:?}: {error}", task.key),
                });
            }
        };
        if reused {
            recorder.finish(ArtifactAttemptOutcome::MemoryCached);
            self.finish_ready(run.id(), task);

            return Ok(());
        }

        // load and select a committed result before running the provider
        let loaded = recorder.span("store_cache", || {
            repository.load_artifact_binding(
                task.revision,
                version,
                Arc::clone(&dependencies),
                Some(recorder.as_ref()),
            )
        });
        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to load cached artifact {:?}: {error}", task.key),
                });
            }
        };
        if loaded {
            recorder.finish(ArtifactAttemptOutcome::StoreCached);
            self.finish_ready(run.id(), task);

            return Ok(());
        }

        // run the provider exactly once over the resolved dependency set
        let attempt = ProviderAttempt::new(repository, task.revision, task.key)
            .with_base(base)
            .with_dependencies(Arc::clone(&dependencies))
            .with_recorder(Arc::clone(&recorder));
        match recorder.span("provider", || self.call_provider(&attempt)) {
            Ok(payload) => {
                let result = recorder.span("complete", || {
                    self.state.repository().complete_artifact(
                        task.revision,
                        version,
                        payload,
                        dependencies,
                        attempt.diagnostics(),
                        attempt.sidecars(),
                        Some(recorder.as_ref()),
                    )
                });
                match result {
                    Ok(_) => {}
                    Err(error) => {
                        recorder.finish(ArtifactAttemptOutcome::Failed);

                        return Err(error.into());
                    }
                }
                recorder.finish(ArtifactAttemptOutcome::Built);
                self.finish_ready(run.id(), task);

                Ok(())
            }
            Err(error) => {
                let result = recorder.span("complete", || {
                    self.fail_provider(&attempt, run.id(), task, version, dependencies, *error)
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);

                result
            }
        }
    }

    /// Route one provider failure into a recorded artifact failure.
    fn fail_provider(
        &self,
        attempt: &ProviderAttempt,
        run: ArtifactRunId,
        task: Task,
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
        error: ProviderError,
    ) -> Result<(), SessionError> {
        let diagnostics = attempt.diagnostics();
        let sidecars = attempt.sidecars();

        match error {
            ProviderError::RequirementFailed { key } => {
                let failure = ArtifactFailure::requirement(key);

                self.fail(
                    run,
                    task,
                    version,
                    dependencies,
                    diagnostics,
                    sidecars,
                    failure,
                )
            }
            ProviderError::Failed { failure } => self.fail(
                run,
                task,
                version,
                dependencies,
                diagnostics,
                sidecars,
                failure,
            ),
            ProviderError::Corrupt { version: corrupt } => Err(SessionError::Internal {
                detail: format!(
                    "failed to provide artifact {:?}: corrupt required artifact: {corrupt:?}",
                    task.key
                ),
            }),
            ProviderError::Internal { message } => {
                let detail = format!("failed to provide artifact {:?}: {message}", task.key);

                Err(SessionError::Internal { detail })
            }
            ProviderError::Blocked { .. } => Err(SessionError::Internal {
                detail: format!(
                    "provider blocked after its dependency set was frozen: {:?}",
                    task.key
                ),
            }),
        }
    }

    /// Mark one ready task terminal and emit its finished event.
    fn finish_ready(&self, run: ArtifactRunId, task: Task) {
        self.scheduler.mark_done(task);

        self.state.emit_event(SessionEvent::TaskFinished {
            run_id: run,
            artifact_key: task.key,
        });
    }

    /// Mark one failed task terminal and emit its failed event.
    fn finish_failed(&self, run: ArtifactRunId, task: Task) {
        self.scheduler.mark_done(task);

        self.state.emit_event(SessionEvent::TaskFailed {
            run_id: run,
            artifact_key: task.key,
        });
    }

    /// Record one artifact failure and emit its failed event.
    fn fail(
        &self,
        run: ArtifactRunId,
        task: Task,
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        self.state.repository().fail_artifact(
            task.revision,
            version,
            dependencies,
            diagnostics,
            sidecars,
            failure,
        )?;

        self.finish_failed(run, task);

        Ok(())
    }

    /// Call the provider that owns one artifact key.
    fn call_provider(&self, attempt: &ProviderAttempt) -> ProviderResult<ArtifactPayload> {
        match attempt.key().provider() {
            ArtifactProvider::Loader => self
                .state
                .provide_loader(attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.state.compiler().provide(attempt),
            ArtifactProvider::Linter => self.state.linter().provide(attempt),
            ArtifactProvider::Index => self.state.indexer().provide(attempt),
        }
    }
}
