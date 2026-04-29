use destack_artifact::{ArtifactFailure, ArtifactKey, ArtifactProvider, ProvideError};

use super::SessionLoop;
use super::task::SessionTask;
use crate::provide::context::SessionContext;
use crate::{Session, SessionError, SessionEvent, SessionRunId};

/// Outcome from one artifact task provider call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionTaskOutcome {
    /// The provider produced a ready payload.
    Ready,
    /// The provider is blocked on these dependency keys.
    Blocked(Vec<ArtifactKey>),
    /// The provider produced a terminal non-ready result.
    Terminal,
}

impl SessionLoop {
    /// Provide one task and return the outcome it reported.
    pub(super) fn provide_task(
        &self,
        session: &Session,
        context: &SessionContext,
        run_id: SessionRunId,
    ) -> Result<SessionTaskOutcome, SessionError> {
        let task = SessionTask::new(context.revision(), context.key());
        let result = self.do_provide(session, context);

        // translate provider protocol into loop actions
        match result {
            // ready payloads are completed by the loop after this call
            Ok(()) => Ok(SessionTaskOutcome::Ready),

            // blocked tasks go back behind the dependencies they discovered
            Err(ProvideError::Blocked { keys }) => {
                // blocked without dependencies would spin forever
                if keys.is_empty() {
                    let error = SessionError::Internal {
                        detail: format!(
                            "artifact provider blocked without dependencies: {:?}",
                            task.key
                        ),
                    };
                    self.fail_task(
                        session,
                        context,
                        run_id,
                        task,
                        ArtifactFailure::internal(error.to_string()),
                    )?;

                    return Err(error);
                }

                Ok(SessionTaskOutcome::Blocked(keys))
            }

            // failed requirements become stored artifact failures
            Err(ProvideError::RequirementFailed { key }) => {
                self.fail_task(
                    session,
                    context,
                    run_id,
                    task,
                    ArtifactFailure::requirement(key),
                )?;

                Ok(SessionTaskOutcome::Terminal)
            }

            // corrupt requirements are store boundary violations
            Err(ProvideError::Corrupt { version }) => {
                self.fail_task(
                    session,
                    context,
                    run_id,
                    task,
                    ArtifactFailure::corrupt(version),
                )?;

                Err(SessionError::Internal {
                    detail: format!(
                        "failed to provide artifact {:?}: corrupt required artifact: {version:?}",
                        task.key
                    ),
                })
            }

            // diagnosed artifacts are terminal but payload free
            Err(ProvideError::Diagnosed) => {
                self.diagnose_task(session, context, run_id, task)?;

                Ok(SessionTaskOutcome::Terminal)
            }

            // internal provider failures stop the session run
            Err(ProvideError::Internal { message }) => {
                self.fail_task(
                    session,
                    context,
                    run_id,
                    task,
                    ArtifactFailure::internal(message.clone()),
                )?;

                Err(SessionError::Internal {
                    detail: format!("failed to provide artifact {:?}: {message}", task.key),
                })
            }
        }
    }

    /// Call the provider that owns one artifact key.
    fn do_provide(&self, session: &Session, context: &SessionContext) -> Result<(), ProvideError> {
        // dispatch by artifact provider family
        match context.key().provider() {
            ArtifactProvider::Source => session
                .provide_source(context)
                .map_err(|error| ProvideError::internal(error.to_string())),
            ArtifactProvider::Compiler => session.compiler().provide(context),
            ArtifactProvider::Linter => session.linter().provide(context),
        }
    }

    /// Fail one task and emit its failure event.
    pub(super) fn fail_task(
        &self,
        session: &Session,
        context: &SessionContext,
        run_id: SessionRunId,
        task: SessionTask,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        let result = context.complete_failed(failure);

        // release waiters before reporting the terminal event
        self.finish(task);

        session.emit_event(SessionEvent::TaskFailed {
            run_id,
            artifact_key: task.key,
        });

        result.map(|_| ())
    }

    /// Diagnose one payload-free task and emit its failure event.
    fn diagnose_task(
        &self,
        session: &Session,
        context: &SessionContext,
        run_id: SessionRunId,
        task: SessionTask,
    ) -> Result<(), SessionError> {
        let result = context.complete_diagnosed();

        // release waiters before reporting the terminal event
        self.finish(task);

        session.emit_event(SessionEvent::TaskFailed {
            run_id,
            artifact_key: task.key,
        });

        result.map(|_| ())
    }
}
