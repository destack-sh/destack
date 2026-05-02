use destack_artifact::ArtifactKey;

use crate::executor::RunId;

/// Event emitted by the session while providing artifacts.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// One outer provide run started.
    RunStarted {
        /// The session run id.
        run_id: RunId,
    },
    /// One session task started.
    TaskStarted {
        /// The session run id.
        run_id: RunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task finished.
    TaskFinished {
        /// The session run id.
        run_id: RunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task failed.
    TaskFailed {
        /// The session run id.
        run_id: RunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One outer provide run finished.
    RunFinished {
        /// The session run id.
        run_id: RunId,
    },
}

/// Session event callback type.
pub type SessionEventHandler = std::sync::Arc<dyn Fn(SessionEvent) + Send + Sync>;
