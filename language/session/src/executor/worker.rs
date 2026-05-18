use std::sync::Arc;

use destack_artifact::{ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactProvider};
use destack_workspace::{ProviderError, ProviderResult};

use super::attempt::ProviderAttempt;
use super::run::{Run, RunId};
use super::scheduler::Scheduler;
use super::task::Task;
use crate::{SessionError, SessionEvent, SessionState};

/// One worker in the session artifact executor.
#[derive(Debug, Clone)]
pub(super) struct Worker {
    /// Shared session state for provider execution.
    pub(super) session: Arc<SessionState>,
    /// Shared scheduler for artifact work.
    pub(super) scheduler: Arc<Scheduler>,
}

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

        // call the provider with one concrete attempt
        let attempt = ProviderAttempt::new(self.session.repository(), task.revision, task.key);
        let result = self.call_provider(&attempt);

        self.handle_provider_result(&attempt, run.id(), task, result)
    }

    /// Route one provider result back into the scheduler.
    fn handle_provider_result(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
        task: Task,
        result: ProviderResult<ArtifactPayload>,
    ) -> Result<(), SessionError> {
        match result {
            Ok(payload) => self.complete_ready(attempt, run, task, payload),
            Err(error) => match *error {
                ProviderError::Blocked { keys } => {
                    self.wait_on_dependencies(attempt, run, task, keys)
                }
                ProviderError::RequirementFailed { key } => {
                    self.fail_task(attempt, run, task, ArtifactFailure::requirement(key))
                }
                ProviderError::Corrupt { version } => {
                    self.fail_task(attempt, run, task, ArtifactFailure::corrupt(version))?;

                    Err(SessionError::Internal {
                        detail: format!(
                            "failed to provide artifact {:?}: corrupt required artifact: {version:?}",
                            task.key
                        ),
                    })
                }
                ProviderError::Failed { failure } => self.fail_task(attempt, run, task, failure),
                ProviderError::Internal { message } => {
                    let detail = format!("failed to provide artifact {:?}: {message}", task.key);

                    self.fail_task(attempt, run, task, ArtifactFailure::internal(message))?;

                    Err(SessionError::Internal { detail })
                }
            },
        }
    }

    /// Put one task to sleep until its dependency artifacts become terminal.
    fn wait_on_dependencies(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
        task: Task,
        keys: Vec<ArtifactKey>,
    ) -> Result<(), SessionError> {
        if keys.is_empty() {
            let error = SessionError::Internal {
                detail: format!(
                    "artifact provider blocked without dependencies: {:?}",
                    task.key
                ),
            };

            self.fail_task(
                attempt,
                run,
                task,
                ArtifactFailure::internal(error.to_string()),
            )?;

            return Err(error);
        }

        let mut dependencies = Vec::new();

        // filter already terminal dependencies
        for key in keys {
            let dependency = Task::new(task.revision, key);
            if self.session.artifact_outcome(dependency)?.is_none() {
                dependencies.push(dependency);
            }
        }

        let result = self.scheduler.wait_on(task, run, dependencies);

        // record scheduler rejected waits as artifact failures
        if let Err(error) = result {
            self.fail_task(
                attempt,
                run,
                task,
                ArtifactFailure::internal(error.to_string()),
            )?;

            return Err(error);
        }

        Ok(())
    }

    /// Complete one ready task and make its terminal state visible.
    fn complete_ready(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
        task: Task,
        payload: ArtifactPayload,
    ) -> Result<(), SessionError> {
        let result = attempt.complete_ready(payload);

        match result {
            // store ready outcome
            Ok(_) => {
                self.scheduler.mark_done(task);

                self.session.emit_event(SessionEvent::TaskFinished {
                    run_id: run,
                    artifact_key: task.key,
                });

                Ok(())
            }

            // store failure outcome so waiters cannot stall
            Err(error) => {
                let failure = ArtifactFailure::internal(error.to_string());
                let failure_result = attempt.complete_failed(failure);

                self.scheduler.mark_done(task);

                self.session.emit_event(SessionEvent::TaskFailed {
                    run_id: run,
                    artifact_key: task.key,
                });

                failure_result?;

                Err(error)
            }
        }
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

    /// Fail one task and emit its failure event.
    fn fail_task(
        &self,
        attempt: &ProviderAttempt,
        run: RunId,
        task: Task,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        let result = attempt.complete_failed(failure);

        self.scheduler.mark_done(task);

        self.session.emit_event(SessionEvent::TaskFailed {
            run_id: run,
            artifact_key: task.key,
        });

        result.map(|_| ())
    }
}
