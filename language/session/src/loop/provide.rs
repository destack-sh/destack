use destack_artifact::{
    ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactProvider, ProvideError,
};

use super::SessionWorker;
use super::task::SessionTask;
use crate::session::SessionContext;
use crate::{SessionError, SessionEvent, SessionRunId};

/// Outcome from one artifact task provider call.
#[derive(Debug, Clone)]
pub(super) enum SessionTaskOutcome {
    /// The provider produced a ready payload.
    Ready(ArtifactPayload),
    /// The provider is blocked on these dependency keys.
    Blocked(Vec<ArtifactKey>),
    /// The provider produced a terminal non-ready result.
    Terminal,
}

impl SessionWorker {
    /// Provide one task and return the outcome it reported.
    pub(super) fn provide_task(
        &self,
        context: &SessionContext,
        run_id: SessionRunId,
    ) -> Result<SessionTaskOutcome, SessionError> {
        let task = SessionTask::new(context.revision(), context.key());
        let result = self.do_provide(context);

        // translate provider protocol into loop actions
        match result {
            // ready payloads are completed by the loop after this call
            Ok(payload) => Ok(SessionTaskOutcome::Ready(payload)),

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
                self.fail_task(context, run_id, task, ArtifactFailure::requirement(key))?;

                Ok(SessionTaskOutcome::Terminal)
            }

            // corrupt requirements are store boundary violations
            Err(ProvideError::Corrupt { version }) => {
                self.fail_task(context, run_id, task, ArtifactFailure::corrupt(version))?;

                Err(SessionError::Internal {
                    detail: format!(
                        "failed to provide artifact {:?}: corrupt required artifact: {version:?}",
                        task.key
                    ),
                })
            }

            // provider failures are terminal artifact failures
            Err(ProvideError::Failed { failure }) => {
                self.fail_task(context, run_id, task, failure)?;

                Ok(SessionTaskOutcome::Terminal)
            }

            // internal provider failures stop the session run
            Err(ProvideError::Internal { message }) => {
                self.fail_task(
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
    fn do_provide(&self, context: &SessionContext) -> Result<ArtifactPayload, ProvideError> {
        // dispatch by artifact provider family
        match context.key().provider() {
            ArtifactProvider::Source => self
                .session
                .provide_source(context)
                .map_err(|error| ProvideError::internal(error.to_string())),
            ArtifactProvider::Compiler => self.session.compiler().provide(context),
            ArtifactProvider::Linter => self.session.linter().provide(context),
        }
    }

    /// Fail one task and emit its failure event.
    pub(super) fn fail_task(
        &self,
        context: &SessionContext,
        run_id: SessionRunId,
        task: SessionTask,
        failure: ArtifactFailure,
    ) -> Result<(), SessionError> {
        let result = context.complete_failed(failure);

        // release waiters before reporting the terminal event
        self.finish(task);

        self.session.emit_event(SessionEvent::TaskFailed {
            run_id,
            artifact_key: task.key,
        });

        result.map(|_| ())
    }
}
