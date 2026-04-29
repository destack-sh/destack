use destack_artifact::ArtifactKey;

/// Id for one session run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SessionRunId(pub u32);

impl std::fmt::Display for SessionRunId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

/// Event emitted by the session while providing artifacts.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// One outer provide run started.
    RunStarted {
        /// The session run id.
        run_id: SessionRunId,
    },
    /// One session task started.
    TaskStarted {
        /// The session run id.
        run_id: SessionRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task finished.
    TaskFinished {
        /// The session run id.
        run_id: SessionRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One session task failed.
    TaskFailed {
        /// The session run id.
        run_id: SessionRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One outer provide run finished.
    RunFinished {
        /// The session run id.
        run_id: SessionRunId,
    },
}

/// Session event callback type.
pub type SessionEventHandler = std::sync::Arc<dyn Fn(SessionEvent) + Send + Sync>;
