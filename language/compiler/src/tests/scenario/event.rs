use std::sync::Arc;

use destack_artifact::ArtifactKey;

/// One internal scenario event emitted by the compiler.
#[derive(Debug, Clone)]
pub(crate) enum CompilerScenarioEvent {
    /// One task finished its work and is about to validate and commit.
    BeforeTaskCommit {
        /// The artifact key being committed.
        artifact_key: ArtifactKey,
    },
}

/// One internal scenario event handler.
pub(crate) type CompilerScenarioEventHandler = Arc<dyn Fn(CompilerScenarioEvent) + Send + Sync>;
