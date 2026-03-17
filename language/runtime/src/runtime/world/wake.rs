use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::{AgentId, RuntimeId};

/// World-timed due work selected by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wake {
    /// Timer wake for one agent event loop.
    AgentTimer {
        /// Runtime that owns the agent.
        runtime_id: RuntimeId,
        /// Agent that owns the timer watch.
        agent_id: AgentId,
        /// Timer that became due.
        timer: Timer,
    },
}

impl Wake {
    /// Return the due wall-clock deadline used for deterministic ordering.
    pub const fn at(&self) -> WorldInstant {
        match self {
            Self::AgentTimer { timer, .. } => WorldInstant::from_nanos(timer.deadline.at),
        }
    }

    /// Return one deterministic wake-kind rank.
    const fn kind_rank(&self) -> u8 {
        match self {
            Self::AgentTimer { .. } => 0,
        }
    }

    /// Return the runtime identifier used for deterministic ordering.
    const fn runtime_rank(&self) -> u64 {
        match self {
            Self::AgentTimer { runtime_id, .. } => runtime_id.0,
        }
    }

    /// Return the agent identifier used for deterministic ordering.
    const fn agent_rank(&self) -> u64 {
        match self {
            Self::AgentTimer { agent_id, .. } => agent_id.0,
        }
    }

    /// Return the final deterministic tie-break rank.
    const fn local_rank(&self) -> u64 {
        match self {
            Self::AgentTimer { timer, .. } => timer.handle.sort_key(),
        }
    }

    /// Return the stable ordering key for this wake.
    pub(crate) const fn sort_key(&self) -> (WorldInstant, u8, u64, u64, u64) {
        (
            self.at(),
            self.kind_rank(),
            self.runtime_rank(),
            self.agent_rank(),
            self.local_rank(),
        )
    }
}
