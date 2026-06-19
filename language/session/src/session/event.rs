use destack_artifact::ArtifactKey;

use crate::executor::ArtifactRunId;

/// Event emitted by the session while providing artifacts.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// One outer provide run started.
    RunStarted {
        /// The artifact run id.
        run_id: ArtifactRunId,
    },
    /// One session task started.
    TaskStarted {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task finished.
    TaskFinished {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task failed.
    TaskFailed {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One outer provide run finished.
    RunFinished {
        /// The artifact run id.
        run_id: ArtifactRunId,
    },
}

/// Session event callback type.
pub type SessionEventHandler = std::sync::Arc<dyn Fn(SessionEvent) + Send + Sync>;
