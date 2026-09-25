use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactProvider, DiagnosticRecord,
};
use tspp_repository::{
    ArtifactAttemptOutcome, ArtifactAttemptRecorder, ArtifactBase, ArtifactPlan, PendingSet,
    ProviderError, ProviderResult, Repository, Revision,
};

use super::attempt::ProviderAttempt;
use super::run::ArtifactRunState;
use super::scheduler::Scheduler;
use super::task::Task;
use crate::{SessionError, SessionEvent, SessionState};

/// One worker in an artifact executor.
#[derive(Debug, Clone)]
pub(super) struct Worker {
    /// The index of this worker in the artifact executor.
    pub(super) index: usize,
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

        let collected = match key.provider() {
            ArtifactProvider::Loader => self
                .collect_loader(&attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.compiler().collect(&attempt),
            ArtifactProvider::Linter => self.linter().collect(&attempt),
            ArtifactProvider::Index => self.indexer().collect(&attempt),
        };

        // preserve failed dynamic reads as terminal requirements
        let mut set = match collected {
            Ok(set) => set,
            Err(error) => match *error {
                ProviderError::RequirementFailed { key } => {
                    let mut set = ArtifactDependencySet::default();
                    set.require(key);

                    set
                }
                error => return Err(Box::new(error)),
            },
        };

        // require every blocked read so the frontier schedules it
        let blocked = attempt.take_blocked();
        if !blocked.is_empty() {
            for artifact_key in blocked {
                set.require(artifact_key);
            }
            set.mark_partial();
        }

        Ok(set)
    }
}

impl Worker {
    /// Convert one caught provider panic into a session error.
    fn panic_error(task: Task, panic: Box<dyn Any + Send>) -> SessionError {
        let message = panic
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("opaque panic payload");

        SessionError::Internal {
            detail: format!("provider panicked for artifact {:?}: {message}", task.key),
        }
    }

    /// Drive the shared scheduler until the executor shuts down.
    pub(super) fn run(&self) {
        loop {
            let Some((run, task, pending_set)) = self.scheduler.claim() else {
                return;
            };

            self.run_task(run, task, pending_set);
        }
    }

    /// Execute one claimed artifact task.
    pub(super) fn run_task(
        &self,
        run: Arc<ArtifactRunState>,
        task: Task,
        pending_set: Option<PendingSet>,
    ) {
        let state = run.session().clone();

        // finish tasks whose revision artifact is already resolved
        match state.artifact_outcome(task) {
            Ok(Some(_)) => {
                self.scheduler.mark_done(task);
            }
            Ok(None) => {
                // contain provider panics so the scheduler never retains an abandoned task
                let provided = catch_unwind(AssertUnwindSafe(|| {
                    self.provide_task(&state, run.as_ref(), task, pending_set)
                }));
                match provided {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => self.scheduler.abort(task, error),
                    Err(panic) => self.scheduler.abort(task, Self::panic_error(task, panic)),
                }
            }
            Err(error) => {
                self.scheduler.abort(task, error);
            }
        }

        self.scheduler.finish_attempt(&run);
    }

    /// Provide one claimed task per the repository's plan.
    pub(super) fn provide_task(
        &self,
        state: &SessionState,
        run: &ArtifactRunState,
        task: Task,
        pending_set: Option<PendingSet>,
    ) -> Result<(), SessionError> {
        let recorder = Arc::new(run.trace().begin(task.key, self.index));

        // expose the session task through events
        run.emit_task(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        // ask the repository what this attempt requires
        let repository = state.repository();
        let mut collect = |base| state.collect_dependencies(task.revision, task.key, base);
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
                    ArtifactOutcome::Ok => self.finish_ready(run, task),
                    ArtifactOutcome::Failed(_) => self.finish_failed(run, task),
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
                let diagnostics = Vec::new();
                let result = recorder.span("commit", || {
                    self.fail(state, task, dependencies, diagnostics, failure)
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);
                result?;
                self.finish_failed(run, task);

                Ok(())
            }
            // run the provider over the exact dependency set
            Ok(ArtifactPlan::Build {
                base,
                dependencies,
                failed: None,
            }) => self.provide(state, run, task, &repository, base, dependencies, &recorder),
            Err(error) => Err(self.fail_internal(&recorder, error.to_string())),
        }
    }

    /// Run the provider once and commit everything it produced and read.
    fn provide(
        &self,
        state: &SessionState,
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
        let provided = recorder.span("provide", || self.call_provider(state, &attempt));

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
                    state.repository().complete_artifact(
                        task.revision,
                        task.key,
                        payload,
                        Arc::clone(&dependencies),
                        attempt.diagnostics(),
                        Some(recorder.as_ref()),
                    )
                });
                if let Err(error) = result {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(error.into());
                }
                recorder.finish(ArtifactAttemptOutcome::Built);
                self.finish_ready(run, task);

                Ok(())
            }
            Err(error) => {
                let result = recorder.span("commit", || {
                    self.fail_provider(state, &attempt, task, dependencies, *error)
                });
                recorder.finish(ArtifactAttemptOutcome::Failed);
                result?;
                self.finish_failed(run, task);

                Ok(())
            }
        }
    }

    /// Park this attempt until its frontier becomes terminal.
    fn park(
        &self,
        task: Task,
        frontier: Vec<ArtifactKey>,
        pending_set: Option<PendingSet>,
        recorder: &Arc<ArtifactAttemptRecorder>,
    ) -> Result<(), SessionError> {
        recorder.record_counter("park.frontier", frontier.len() as u64);
        let frontier = frontier
            .into_iter()
            .map(|key| Task::new(task.session, task.revision, key))
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
        state: &SessionState,
        attempt: &ProviderAttempt,
        task: Task,
        dependencies: Arc<[ArtifactDependency]>,
        error: ProviderError,
    ) -> Result<(), SessionError> {
        let diagnostics = attempt.diagnostics();

        match error {
            ProviderError::RequirementFailed { key } => {
                let failure = ArtifactFailure::requirement(key);

                self.fail(state, task, dependencies, diagnostics, failure)
            }
            ProviderError::Failed { failure } => {
                self.fail(state, task, dependencies, diagnostics, failure)
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

    /// Emit one completed task before releasing its dependents.
    fn finish_ready(&self, run: &ArtifactRunState, task: Task) {
        run.emit_task(SessionEvent::TaskFinished {
            run_id: run.id(),
            artifact_key: task.key,
        });

        self.scheduler.mark_done(task);
    }

    /// Emit one failed task before releasing its dependents.
    fn finish_failed(&self, run: &ArtifactRunState, task: Task) {
        run.emit_task(SessionEvent::TaskFailed {
            run_id: run.id(),
            artifact_key: task.key,
        });

        self.scheduler.mark_done(task);
    }

    /// Commit one artifact failure.
    fn fail(
        &self,
        state: &SessionState,
        task: Task,
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: Vec<DiagnosticRecord>,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        state.repository().fail_artifact(
            task.revision,
            task.key,
            dependencies,
            diagnostics,
            failure,
        )?;

        Ok(())
    }

    /// Call the provider that owns one artifact key.
    fn call_provider(
        &self,
        state: &SessionState,
        attempt: &ProviderAttempt,
    ) -> ProviderResult<ArtifactPayload> {
        match attempt.key().provider() {
            ArtifactProvider::Loader => state
                .provide_loader(attempt)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => state.compiler().provide(attempt),
            ArtifactProvider::Linter => state.linter().provide(attempt),
            ArtifactProvider::Index => state.indexer().provide(attempt),
        }
    }
}
