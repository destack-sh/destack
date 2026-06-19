use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactPayload, ArtifactProvider,
    ArtifactVersion,
};
use destack_repository::{
    ArtifactAttemptOutcome, Collected, Collector, ProviderError, ProviderResult, Revision,
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

impl Collector for SessionState {
    /// Collect the dependency closure for the system that owns one key.
    fn collect(
        &self,
        revision: Revision,
        key: destack_artifact::ArtifactKey,
    ) -> ProviderResult<ArtifactDependencySet> {
        let attempt = ProviderAttempt::new(self.repository(), revision, key);

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
            let Some((run, task)) = self.scheduler.claim() else {
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
            if let Err(error) = self.provide_task(run.as_ref(), task) {
                run.abort(error);
                self.scheduler.notify();
            }
        }
    }

    /// Provide one claimed task.
    pub(super) fn provide_task(&self, run: &ArtifactRun, task: Task) -> Result<(), SessionError> {
        let recorder = Arc::new(run.trace().begin(task.key, self.index));

        // expose the session task through events
        self.session.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        let repository = self.session.repository();

        // freeze the dependency closure through the shared engine
        let collected = recorder.span("collect", || {
            repository.collect_closure(self.session.as_ref(), task.revision, task.key)
        });
        let collected = match collected {
            Ok(collected) => collected,
            Err(error) => {
                recorder.finish(ArtifactAttemptOutcome::Failed);

                return Err(SessionError::Internal {
                    detail: format!("failed to collect artifact {:?}: {error}", task.key),
                });
            }
        };
        let (dependencies, failed) = match collected {
            Collected::Waiting { frontier, .. } => {
                recorder.record_counter("frontier", frontier.len() as u64);
                let frontier = frontier
                    .into_iter()
                    .map(|key| Task::new(task.revision, key))
                    .collect();
                let result = recorder.span("scheduler", || self.wait_on(run.id(), task, frontier));
                let outcome = if result.is_ok() {
                    ArtifactAttemptOutcome::Blocked
                } else {
                    ArtifactAttemptOutcome::Failed
                };
                recorder.finish(outcome);

                return result;
            }
            Collected::Frozen {
                dependencies,
                failed,
            } => (dependencies, failed),
        };
        // the version is fixed by the frozen closure before any provider runs
        let version = recorder.span("version", || {
            ArtifactVersion::new(
                task.key,
                repository.build_fingerprint(),
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
                    dependencies,
                    DiagnosticCollection::new(),
                    Vec::new(),
                    ArtifactFailure::requirement(failed_dependency),
                )
            });
            recorder.finish(ArtifactAttemptOutcome::Failed);

            return result;
        }

        // reuse a committed payload when the closure already produced this version
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
            let result = recorder.span("scheduler", || self.finish_ready(run.id(), task));
            let outcome = if result.is_ok() {
                ArtifactAttemptOutcome::MemoryCached
            } else {
                ArtifactAttemptOutcome::Failed
            };
            recorder.finish(outcome);

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
            let result = recorder.span("scheduler", || self.finish_ready(run.id(), task));
            let outcome = if result.is_ok() {
                ArtifactAttemptOutcome::StoreCached
            } else {
                ArtifactAttemptOutcome::Failed
            };
            recorder.finish(outcome);

            return result;
        }

        // run the provider exactly once over the frozen closure
        let attempt = ProviderAttempt::new(repository, task.revision, task.key)
            .with_recorder(Arc::clone(&recorder));
        match recorder.span("provider", || self.call_provider(&attempt)) {
            Ok(payload) => {
                let result = recorder.span("publish", || {
                    self.publish_artifact(&attempt, task, version, dependencies, payload)
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
                let result = recorder.span("scheduler", || self.finish_ready(run.id(), task));
                let outcome = if result.is_ok() {
                    ArtifactAttemptOutcome::Built
                } else {
                    ArtifactAttemptOutcome::Failed
                };
                recorder.finish(outcome);

                result
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

    /// Park one task until its unready dependency frontier becomes terminal.
    fn wait_on(
        &self,
        run: ArtifactRunId,
        task: Task,
        frontier: Vec<Task>,
    ) -> Result<(), SessionError> {
        let repository = self.session.repository();
        let result = self.scheduler.wait_on(task, run, frontier);

        // a rejected wait records the artifact as failed so waiters cannot stall
        if let Err(error) = result {
            let version =
                ArtifactVersion::new(task.key, repository.build_fingerprint(), Vec::new());

            self.fail(
                run,
                task,
                version,
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
        dependencies: Vec<ArtifactDependency>,
        payload: ArtifactPayload,
    ) -> Result<(), SessionError> {
        // record the ready payload and make its terminal state visible
        self.session.repository().publish_artifact(
            task.revision,
            version,
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
                dependencies,
                diagnostics,
                sidecars,
                ArtifactFailure::requirement(key),
            ),
            ProviderError::Failed { failure } => self.fail(
                run,
                task,
                version,
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
                    dependencies,
                    diagnostics,
                    sidecars,
                    ArtifactFailure::internal(message),
                )?;

                Err(SessionError::Internal { detail })
            }
            ProviderError::Blocked { .. } => Err(SessionError::Internal {
                detail: format!(
                    "provider blocked after its dependency closure was frozen: {:?}",
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
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<destack_artifact::ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        self.session.repository().fail_artifact(
            task.revision,
            version,
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
