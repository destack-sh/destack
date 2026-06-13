use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactPayload, ArtifactProvider,
    ArtifactVersion,
};
use destack_repository::{
    Collected, Collector, ProviderError, ProviderResult, Revision, TraceOutcome,
};
use destack_source::DiagnosticCollection;

use super::attempt::ProviderAttempt;
use super::run::{Run, RunId};
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

            // artifact store truth wins over stale scheduler entries
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
    fn provide_task(&self, run: &Run, task: Task) -> Result<(), SessionError> {
        // expose the session task through events
        self.session.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        let repository = self.session.repository();

        // freeze the dependency closure through the shared engine
        let collected = repository
            .collect_closure(self.session.as_ref(), task.revision, task.key)
            .map_err(|error| SessionError::Internal {
                detail: format!("failed to collect artifact {:?}: {error}", task.key),
            })?;
        let (dependencies, failed) = match collected {
            Collected::Waiting { frontier, .. } => {
                let frontier = frontier
                    .into_iter()
                    .map(|key| Task::new(task.revision, key))
                    .collect();

                return self.wait_on(run.id(), task, frontier);
            }
            Collected::Frozen {
                dependencies,
                failed,
            } => (dependencies, failed),
        };

        // the version is fixed by the frozen closure before any provider runs
        let version = ArtifactVersion::new(task.key, dependencies.iter().cloned());

        // fails this artifact immediately on a poisoned dependency
        if let Some(failed_dependency) = failed {
            return self.fail(
                run.id(),
                task,
                version,
                dependencies,
                DiagnosticCollection::new(),
                Vec::new(),
                ArtifactFailure::requirement(failed_dependency),
            );
        }

        // reuse a committed payload when the closure already produced this version
        if repository.artifact_store().outcome(&version).is_some() {
            repository
                .bind_artifact(task.revision, version)
                .map_err(|error| SessionError::Internal {
                    detail: format!("failed to bind reused artifact {:?}: {error}", task.key),
                })?;

            return self.finish_ready(run.id(), task);
        }

        // run the provider exactly once over the frozen closure
        let tracer = Arc::new(run.trace().begin(task.key, self.index));
        let attempt = ProviderAttempt::new(repository, task.revision, task.key)
            .with_tracer(Arc::clone(&tracer));
        let result = self.call_provider(&attempt);
        let outcome = match &result {
            Ok(_) => TraceOutcome::Ready,
            Err(_) => TraceOutcome::Failed,
        };
        tracer.finish(outcome);

        self.complete(&attempt, run.id(), task, version, dependencies, result)
    }

    /// Park one task until its unready dependency frontier becomes terminal.
    fn wait_on(&self, run: RunId, task: Task, frontier: Vec<Task>) -> Result<(), SessionError> {
        let result = self.scheduler.wait_on(task, run, frontier);

        // a rejected wait records the artifact as failed so waiters cannot stall
        if let Err(error) = result {
            let version = ArtifactVersion::new(task.key, Vec::new());

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

    /// Complete one task from its provider result.
    fn complete(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
        task: Task,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        result: ProviderResult<ArtifactPayload>,
    ) -> Result<(), SessionError> {
        let payload = match result {
            Ok(payload) => payload,
            Err(error) => {
                return self.fail_provider(attempt, run, task, version, dependencies, *error);
            }
        };

        // record the ready payload and make its terminal state visible
        self.session.repository().complete_artifact(
            task.revision,
            version,
            payload,
            dependencies,
            attempt.diagnostics(),
            attempt.sidecars(),
        )?;

        self.finish_ready(run, task)
    }

    /// Route one provider failure into a recorded artifact failure.
    fn fail_provider(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
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
    fn finish_ready(&self, run: RunId, task: Task) -> Result<(), SessionError> {
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
        run: RunId,
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
