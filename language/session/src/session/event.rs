use std::time::Duration;

use tspp_artifact::ArtifactKey;
use tspp_repository::Revision;

use crate::executor::{ArtifactPriority, ArtifactRunId};

/// Event emitted for one artifact run.
#[derive(Debug, Clone)]
pub enum ArtifactRunEvent {
    /// One artifact run started.
    Started {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The immutable revision read by the run.
        revision: Revision,
        /// The scheduling priority selected for the run.
        priority: ArtifactPriority,
        /// The initial artifact roots.
        artifact_keys: Vec<ArtifactKey>,
    },
    /// One active run requested artifact roots.
    Required {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The requested artifact roots.
        artifact_keys: Vec<ArtifactKey>,
    },
    /// One artifact run finished.
    Finished {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// Whether the run was cancelled before finishing.
        is_cancelled: bool,
        /// Whether the run stopped after an executor failure.
        is_aborted: bool,
        /// Time from run creation through completion.
        elapsed: Duration,
    },
}

/// Event emitted by the session while providing artifacts.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// One artifact run lifecycle event.
    Run(ArtifactRunEvent),
    /// One artifact task started.
    TaskStarted {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One artifact task finished.
    TaskFinished {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One artifact task failed.
    TaskFailed {
        /// The artifact run id.
        run_id: ArtifactRunId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
}

/// Thread-safe handler for session events.
pub type SessionEventHandler = std::sync::Arc<dyn Fn(SessionEvent) + Send + Sync>;
