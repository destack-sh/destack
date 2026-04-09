use std::sync::Arc;
use std::time::Duration;

use destack_artifact::ArtifactKey;

/// Id for one session owned artifact provide attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProvideId(pub u32);

impl std::fmt::Display for ProvideId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

/// Summary of one session owned provide run.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionStats {
    /// Total artifact attempts started in the run.
    pub started: usize,
    /// Total artifact attempts completed in the run.
    pub completed: usize,
    /// Total artifact attempts that failed in the run.
    pub failed: usize,
    /// Total artifact attempts that yielded requirements in the run.
    pub yielded: usize,
    /// Total artifact attempts that crossed the slow threshold in the run.
    pub slow: usize,
}

/// Event emitted by the session while providing artifacts.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// One outer provide run started.
    RunStarted,
    /// One artifact provide attempt started.
    ArtifactStarted {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One artifact provide attempt completed.
    ArtifactCompleted {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
        /// The elapsed attempt time.
        elapsed: Duration,
    },
    /// One artifact provide attempt failed.
    ArtifactFailed {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One artifact provide attempt yielded requirements.
    ArtifactYielded {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
    },
    /// One artifact provide attempt exceeded the slow threshold.
    ArtifactSlow {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
        /// The elapsed attempt time.
        elapsed: Duration,
    },
    /// One outer provide run finished.
    RunFinished {
        /// Summary of the completed run.
        stats: SessionStats,
    },
}

/// Session event callback type.
pub type SessionEventHandler = Arc<dyn Fn(SessionEvent) + Send + Sync>;

/// One structured observation emitted during a session owned provide run.
#[derive(Debug, Clone)]
pub enum SessionObservation {
    /// One compiler timing tag sample for a provide attempt.
    CompilerTimingTag {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
        /// The timing tag name.
        name: &'static str,
        /// The inclusive duration for this sample.
        duration: Duration,
        /// The number of aggregated samples represented here.
        sample_count: usize,
    },
    /// One parser timing tag sample for a provide attempt.
    ParserTimingTag {
        /// The attempt id.
        provide_id: ProvideId,
        /// The root or dependency artifact key.
        artifact_key: ArtifactKey,
        /// The timing tag name.
        name: &'static str,
        /// The inclusive duration for this sample.
        duration: Duration,
        /// The self duration for this sample.
        self_duration: Duration,
        /// The number of aggregated samples represented here.
        sample_count: usize,
    },
}

/// Session observation callback type.
pub type SessionObservationHandler = Arc<dyn Fn(SessionObservation) + Send + Sync>;
