use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactPayload, ArtifactProvider,
    ArtifactSidecar, ArtifactVersion,
};
use destack_repository::{
    ArtifactAttemptOutcome, DependencySetResolution, ProviderError, ProviderResult, Revision,
};
use destack_source::DiagnosticCollection;

use super::attempt::ProviderAttempt;
use super::run::{ArtifactRun, ArtifactRunId};
use super::scheduler::Scheduler;
use super::task::Task;
use crate::{SessionError, SessionEvent, SessionState};

/// One worker in the session artifact executor.
#[derive(Debug, Clone)]
pub(super) struct Worker {
    /// The index of this worker in the session pool.
    pub(super) index: usize,
    /// Shared session state for provider execution.
    pub(super) session: Arc<SessionState>,
    /// Shared scheduler for artifact work.
    pub(super) scheduler: Arc<Scheduler>,
}

impl SessionState {
    /// Collect the dependency set for the system that owns one key.
    fn collect_dependencies(
        &self,
        revision: Revision,
        key: destack_artifact::ArtifactKey,
        base: Option<destack_repository::ArtifactBase>,
    ) -> ProviderResult<ArtifactDependencySet> {
        let attempt = ProviderAttempt::new(self.repository(), revision, key).with_base(base);

        match key.provider() {
            ArtifactProvider::Loader => self
                .collect_loader(&attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.compiler().collect(&attempt),
            ArtifactProvider::Linter => self.linter().collect(&attempt),
            ArtifactProvider::Query => self.query().collect(&attempt),
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

            // artifact table truth wins over stale scheduler entries
            match self.session.artifact_outcome(task) {
                Ok(Some(_)) => {
                    self.scheduler.mark_done(task);

                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    run.abort(error);
                    self.scheduler.mark_done(task);

                    continue;
                }
            }

            // execute claimed work for the pinned task revision
            if let Err(error) = self.provide_task(run.as_ref(), task, pending_set) {
                run.abort(error);
                self.scheduler.notify();
            }
        }
    }

    /// Provide one claimed task.
    pub(super) fn provide_task(
        &self,
        run: &ArtifactRun,
        task: Task,
        pending_set: Option<ArtifactDependencySet>,
    ) -> Result<(), SessionError> {
        let recorder = Arc::new(run.trace().begin(task.key, self.index));

        // expose the session task through events
        self.session.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        let repository = self.session.repository();
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
                self.session
                    .collect_dependencies(task.revision, task.key, base)
            })
        };
        let (base_version, dependencies, failed) = loop {
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
                repository.resolve_dependency_set(task.revision, dependency_set)
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
                        self.session
                            .collect_dependencies(task.revision, task.key, base)
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
                        self.wait_on(run.id(), task, frontier, pending_set)
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
                    base,
                    dependencies,
                    failed,
                } => break (base, dependencies, failed),
            }
        };
        // the version is fixed by the resolved dependency set before any provider runs
        let version = recorder.span("version", || {
            ArtifactVersion::new(
                task.key,
                repository.build_fingerprint(),
                base_version,
                dependencies.iter().cloned(),
            )
        });

        // fails this artifact immediately on a poisoned dependency
        if let Some(failed_dependency) = failed {
            let result = recorder.span("complete", || {
                self.fail(
                    run.id(),
                    task,
                    version,
                    base_version,
                    dependencies,
                    DiagnosticCollection::new(),
                    Vec::new(),
                    ArtifactFailure::requirement(failed_dependency),
                )
            });
            recorder.finish(ArtifactAttemptOutcome::Failed);

            return result;
        }

        // reuse a committed payload when the dependency set already produced this version
        if recorder.span("memory_cache", || {
            repository.artifact_table().outcome(&version).is_some()
        }) {
            let result = recorder.span("bind", || repository.bind_artifact(task.revision, version));
            if let Err(error) = result {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to bind reused artifact {:?}: {error}", task.key),
                });
            }
            recorder.finish(ArtifactAttemptOutcome::MemoryCached);
            let result = self.finish_ready(run.id(), task);

            return result;
        }

        // load a committed record before running the provider
        let loaded = recorder.span("store_cache", || {
            repository.load_artifact(task.revision, version)
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
            let result = self.finish_ready(run.id(), task);

            return result;
        }

        // run the provider exactly once over the resolved dependency set
        let attempt = ProviderAttempt::new(repository, task.revision, task.key)
            .with_base(base)
            .with_recorder(Arc::clone(&recorder));
        match recorder.span("provider", || self.call_provider(&attempt)) {
            Ok(payload) => {
                let result = recorder.span("publish", || {
                    self.publish_artifact(
                        &attempt,
                        task,
                        version,
                        base_version,
                        dependencies,
                        payload,
                    )
                });
                if let Err(error) = result {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(error);
                }
                let result = recorder.span("store", || {
                    self.session.repository().store_artifact(version)
                });
                if let Err(error) = result {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(error.into());
                }
                recorder.finish(ArtifactAttemptOutcome::Built);

                self.finish_ready(run.id(), task)
            }
            Err(error) => {
                let result = recorder.span("complete", || {
                    self.fail_provider(
                        &attempt,
                        run.id(),
                        task,
                        version,
                        base_version,
                        dependencies,
                        *error,
                    )
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);

                result
            }
        }
    }

    /// Park one task until its unready dependency frontier becomes terminal.
    fn wait_on(
        &self,
        run: ArtifactRunId,
        task: Task,
        frontier: Vec<Task>,
        pending_set: Option<ArtifactDependencySet>,
    ) -> Result<(), SessionError> {
        let repository = self.session.repository();
        let result = self.scheduler.wait_on(task, run, frontier, pending_set);

        // a rejected wait records the artifact as failed so waiters cannot stall
        if let Err(error) = result {
            let version =
                ArtifactVersion::new(task.key, repository.build_fingerprint(), None, Vec::new());

            self.fail(
                run,
                task,
                version,
                None,
                Vec::new(),
                DiagnosticCollection::new(),
                Vec::new(),
                ArtifactFailure::internal(error.to_string()),
            )?;

            return Err(error);
        }

        Ok(())
    }

    /// Publish one ready artifact payload.
    fn publish_artifact(
        &self,
        attempt: &ProviderAttempt,
        task: Task,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        payload: ArtifactPayload,
    ) -> Result<(), SessionError> {
        // record the ready payload and make its terminal state visible
        self.session.repository().publish_artifact(
            task.revision,
            version,
            base,
            payload,
            dependencies,
            attempt.diagnostics(),
            attempt.sidecars(),
        )?;

        Ok(())
    }

    /// Route one provider failure into a recorded artifact failure.
    fn fail_provider(
        &self,
        attempt: &ProviderAttempt,
        run: ArtifactRunId,
        task: Task,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        error: ProviderError,
    ) -> Result<(), SessionError> {
        let diagnostics = attempt.diagnostics();
        let sidecars = attempt.sidecars();

        match error {
            ProviderError::RequirementFailed { key } => self.fail(
                run,
                task,
                version,
                base,
                dependencies,
                diagnostics,
                sidecars,
                ArtifactFailure::requirement(key),
            ),
            ProviderError::Failed { failure } => self.fail(
                run,
                task,
                version,
                base,
                dependencies,
                diagnostics,
                sidecars,
                failure,
            ),
            ProviderError::Corrupt { version: corrupt } => {
                self.fail(
                    run,
                    task,
                    version,
                    base,
                    dependencies,
                    diagnostics,
                    sidecars,
                    ArtifactFailure::corrupt(corrupt),
                )?;

                Err(SessionError::Internal {
                    detail: format!(
                        "failed to provide artifact {:?}: corrupt required artifact: {corrupt:?}",
                        task.key
                    ),
                })
            }
            ProviderError::Internal { message } => {
                let detail = format!("failed to provide artifact {:?}: {message}", task.key);

                self.fail(
                    run,
                    task,
                    version,
                    base,
                    dependencies,
                    diagnostics,
                    sidecars,
                    ArtifactFailure::internal(message),
                )?;

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
    fn finish_ready(&self, run: ArtifactRunId, task: Task) -> Result<(), SessionError> {
        self.scheduler.mark_done(task);

        self.session.emit_event(SessionEvent::TaskFinished {
            run_id: run,
            artifact_key: task.key,
        });

        Ok(())
    }

    /// Record one artifact failure and emit its failed event.
    fn fail(
        &self,
        run: ArtifactRunId,
        task: Task,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        self.session.repository().fail_artifact(
            task.revision,
            version,
            base,
            dependencies,
            diagnostics,
            sidecars,
            failure,
        )?;
        self.scheduler.mark_done(task);

        self.session.emit_event(SessionEvent::TaskFailed {
            run_id: run,
            artifact_key: task.key,
        });

        Ok(())
    }

    /// Call the provider that owns one artifact key.
    fn call_provider(&self, attempt: &ProviderAttempt) -> ProviderResult<ArtifactPayload> {
        match attempt.key().provider() {
            ArtifactProvider::Loader => self
                .session
                .provide_loader(attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.session.compiler().provide(attempt),
            ArtifactProvider::Linter => self.session.linter().provide(attempt),
            ArtifactProvider::Query => self.session.query().provide(attempt),
        }
    }
}
