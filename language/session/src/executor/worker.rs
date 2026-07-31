use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactProvider, ArtifactSidecar,
};
use destack_repository::{
    ArtifactAttemptOutcome, ArtifactAttemptRecorder, ArtifactBase, ArtifactPlan, ProviderError,
    ProviderResult, Repository, Revision,
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
        base: Option<Arc<ArtifactBase>>,
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

    /// Provide one claimed task per the repository's plan.
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

        // ask the repository what this attempt requires
        let repository = self.state.repository();
        let mut collect = |base| {
            self.state
                .collect_dependencies(task.revision, task.key, base)
        };
        let plan = repository.plan_artifact(
            task.revision,
            task.key,
            pending_set,
            recorder.as_ref(),
            &mut collect,
        );

        match plan {
            // finish with the terminal result the revision already binds
            Ok(ArtifactPlan::Done { outcome, attempt }) => {
                recorder.finish(attempt);
                match outcome {
                    ArtifactOutcome::Ok => self.finish_ready(run.id(), task),
                    ArtifactOutcome::Failed(_) => self.finish_failed(run.id(), task),
                }

                Ok(())
            }
            // wait until the declared frontier becomes terminal
            Ok(ArtifactPlan::Park {
                frontier,
                pending_set,
            }) => self.park(task, frontier, pending_set, &recorder),
            // fail immediately on a poisoned dependency
            Ok(ArtifactPlan::Build {
                dependencies,
                failed: Some(failed_dependency),
                ..
            }) => {
                let failure = ArtifactFailure::requirement(failed_dependency);
                let diagnostics = DiagnosticCollection::new();
                let result = recorder.span("commit", || {
                    self.fail(
                        run.id(),
                        task,
                        dependencies,
                        diagnostics,
                        Vec::new(),
                        failure,
                    )
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);

                result
            }
            // run the provider over the exact dependency set
            Ok(ArtifactPlan::Build {
                base,
                dependencies,
                failed: None,
            }) => self.provide(run, task, &repository, base, dependencies, &recorder),
            Err(error) => Err(self.fail_internal(&recorder, error.to_string())),
        }
    }

    /// Run the provider once and commit everything it produced and read.
    fn provide(
        &self,
        run: &ArtifactRunState,
        task: Task,
        repository: &Arc<Repository>,
        base: Option<Arc<ArtifactBase>>,
        dependencies: Arc<[ArtifactDependency]>,
        recorder: &Arc<ArtifactAttemptRecorder>,
    ) -> Result<(), SessionError> {
        // run the provider exactly once over the resolved dependency set
        let attempt = ProviderAttempt::new(Arc::clone(repository), task.revision, task.key)
            .with_base(base)
            .with_dependencies(Arc::clone(&dependencies))
            .with_recorder(Arc::clone(recorder));
        let provided = recorder.span("provide", || self.call_provider(&attempt));

        // park and retry when execution read an artifact that was not ready
        if let Err(error) = &provided
            && let ProviderError::Blocked { keys } = error.as_ref()
        {
            return self.park(task, keys.clone(), None, recorder);
        }

        // complete the dependency set with everything the provider read
        let mut reads = attempt.take_reads();
        recorder.record_reads(&reads);
        let dependencies = if reads.is_empty() {
            dependencies
        } else {
            // deduplicate repeated reads of one value
            reads.sort_unstable();
            reads.dedup();

            let mut complete = dependencies.to_vec();
            complete.extend(reads);

            Arc::<[ArtifactDependency]>::from(complete)
        };

        match provided {
            Ok(payload) => {
                let result = recorder.span("commit", || {
                    self.state.repository().complete_artifact(
                        task.revision,
                        task.key,
                        payload,
                        Arc::clone(&dependencies),
                        attempt.diagnostics(),
                        attempt.sidecars(),
                        Some(recorder.as_ref()),
                    )
                });
                if let Err(error) = result {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(error.into());
                }
                recorder.finish(ArtifactAttemptOutcome::Built);
                self.finish_ready(run.id(), task);

                Ok(())
            }
            Err(error) => {
                let result = recorder.span("commit", || {
                    self.fail_provider(&attempt, run.id(), task, dependencies, *error)
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);

                result
            }
        }
    }

    /// Park this attempt until its frontier becomes terminal.
    fn park(
        &self,
        task: Task,
        frontier: Vec<ArtifactKey>,
        pending_set: Option<ArtifactDependencySet>,
        recorder: &Arc<ArtifactAttemptRecorder>,
    ) -> Result<(), SessionError> {
        recorder.record_counter("park.frontier", frontier.len() as u64);
        let frontier = frontier
            .into_iter()
            .map(|key| Task::new(task.revision, key))
            .collect();
        let result = recorder.span("park", || {
            self.scheduler.wait_on(task, frontier, pending_set)
        });
        let outcome = if result.is_ok() {
            ArtifactAttemptOutcome::Parked
        } else {
            ArtifactAttemptOutcome::Failed
        };
        recorder.finish(outcome);

        result
    }

    /// Fail this attempt with one internal error.
    fn fail_internal(&self, recorder: &ArtifactAttemptRecorder, detail: String) -> SessionError {
        recorder.finish(ArtifactAttemptOutcome::Failed);

        SessionError::Internal { detail }
    }

    /// Route one provider failure into a recorded artifact failure.
    fn fail_provider(
        &self,
        attempt: &ProviderAttempt,
        run: ArtifactRunId,
        task: Task,
        dependencies: Arc<[ArtifactDependency]>,
        error: ProviderError,
    ) -> Result<(), SessionError> {
        let diagnostics = attempt.diagnostics();
        let sidecars = attempt.sidecars();

        match error {
            ProviderError::RequirementFailed { key } => {
                let failure = ArtifactFailure::requirement(key);

                self.fail(run, task, dependencies, diagnostics, sidecars, failure)
            }
            ProviderError::Failed { failure } => {
                self.fail(run, task, dependencies, diagnostics, sidecars, failure)
            }
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
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        self.state.repository().fail_artifact(
            task.revision,
            task.key,
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
