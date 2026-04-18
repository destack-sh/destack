use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::{RuntimeId, WorkerId};

/// World-timed due work selected by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wake {
    /// Timer wake for one worker event loop.
    WorkerTimer {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that owns the timer watch.
        worker_id: WorkerId,
        /// Timer that became due.
        timer: Timer,
    },
}

impl Wake {
    /// Return the due wall-clock deadline used for deterministic ordering.
    pub const fn at(&self) -> WorldInstant {
        match self {
            Self::WorkerTimer { timer, .. } => WorldInstant::from_nanos(timer.deadline.at),
        }
    }

    /// Return one deterministic wake-kind rank.
    const fn kind_rank(&self) -> u8 {
        match self {
            Self::WorkerTimer { .. } => 0,
        }
    }

    /// Return the runtime identifier used for deterministic ordering.
    const fn runtime_rank(&self) -> u64 {
        match self {
            Self::WorkerTimer { runtime_id, .. } => runtime_id.0,
        }
    }

    /// Return the worker identifier used for deterministic ordering.
    const fn worker_rank(&self) -> u64 {
        match self {
            Self::WorkerTimer { worker_id, .. } => worker_id.0,
        }
    }

    /// Return the final deterministic tie-break rank.
    const fn local_rank(&self) -> u64 {
        match self {
            Self::WorkerTimer { timer, .. } => timer.handle.sort_key(),
        }
    }

    /// Return the stable ordering key for this wake.
    pub(crate) const fn sort_key(&self) -> (WorldInstant, u8, u64, u64, u64) {
        (
            self.at(),
            self.kind_rank(),
            self.runtime_rank(),
            self.worker_rank(),
            self.local_rank(),
        )
    }
}
