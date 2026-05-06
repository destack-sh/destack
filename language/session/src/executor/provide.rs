use destack_artifact::{ArtifactFailure, ArtifactPayload, ArtifactProvider};
use destack_workspace::{ProviderError, ProviderResult};

use super::Worker;
use super::task::Task;
use crate::{RunId, SessionError, SessionEvent, SessionProviderContext};

impl Worker {
    /// Call the provider that owns one artifact key.
    pub(super) fn call_provider(
        &self,
        context: &SessionProviderContext,
    ) -> ProviderResult<ArtifactPayload> {
        // dispatch by artifact provider family
        match context.artifact_key().provider() {
            ArtifactProvider::Source => self
                .session
                .provide_source(context)
                .map_err(|error| ProviderError::internal(error.to_string()).into()),
            ArtifactProvider::Compiler => self.session.compiler().provide(context),
            ArtifactProvider::Linter => self.session.linter().provide(context),
            ArtifactProvider::Query => self.session.query().provide(context),
        }
    }

    /// Fail one task and emit its failure event.
    pub(super) fn fail_task(
        &self,
        context: &SessionProviderContext,
        run_id: RunId,
        task: Task,
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
