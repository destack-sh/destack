use serde::{Deserialize, Serialize};

use crate::host::binding::BindingDescriptor;
use crate::runtime::WorkerId;
use crate::world::policy::Attempt;

/// Runtime event kind that can evaluate scenario rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeEventKind {
    /// Before invoking one binding implementation.
    BindingBefore,
    /// After invoking one binding implementation.
    BindingAfter,
    /// One task became ready to run.
    TaskReady,
    /// One task started running.
    TaskStart,
    /// One timer fired.
    TimerFire,
    /// One ingress event entered the worker loop.
    IngressReady,
    /// One clock was read.
    ClockRead,
    /// Random data was read.
    RandomRead,
}

/// Stable identifier for one binding call scenario event pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScenarioCallId(pub u64);

/// Runtime event payload emitted by one worker.
#[derive(Debug, Clone, Copy)]
pub enum RuntimeEvent {
    /// Event fired before invoking one binding.
    BindingBefore {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: ScenarioCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired after invoking one binding.
    BindingAfter {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: ScenarioCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one task becomes ready.
    TaskReady {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one task starts running.
    TaskStart {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one timer fires.
    TimerFire {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one ingress event becomes ready.
    IngressReady {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one clock is read.
    ClockRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when random data is read.
    RandomRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
}

impl RuntimeEvent {
    /// Return the event kind for this event.
    pub(crate) const fn kind(&self) -> RuntimeEventKind {
        match self {
            Self::BindingBefore { .. } => RuntimeEventKind::BindingBefore,
            Self::BindingAfter { .. } => RuntimeEventKind::BindingAfter,
            Self::TaskReady { .. } => RuntimeEventKind::TaskReady,
            Self::TaskStart { .. } => RuntimeEventKind::TaskStart,
            Self::TimerFire { .. } => RuntimeEventKind::TimerFire,
            Self::IngressReady { .. } => RuntimeEventKind::IngressReady,
            Self::ClockRead { .. } => RuntimeEventKind::ClockRead,
            Self::RandomRead { .. } => RuntimeEventKind::RandomRead,
        }
    }

    /// Return the worker identifier for this event.
    pub(crate) const fn worker_id(&self) -> WorkerId {
        match self {
            Self::BindingBefore { worker_id, .. }
            | Self::BindingAfter { worker_id, .. }
            | Self::TaskReady { worker_id, .. }
            | Self::TaskStart { worker_id, .. }
            | Self::TimerFire { worker_id, .. }
            | Self::IngressReady { worker_id, .. }
            | Self::ClockRead { worker_id, .. }
            | Self::RandomRead { worker_id, .. } => *worker_id,
        }
    }

    /// Return one binding descriptor when present.
    pub(crate) const fn binding_descriptor(&self) -> Option<BindingDescriptor> {
        match self {
            Self::BindingBefore { descriptor, .. } | Self::BindingAfter { descriptor, .. } => {
                Some(*descriptor)
            }
            _ => None,
        }
    }

    /// Return one call id when this event is one binding call.
    pub(crate) const fn call_id(&self) -> Option<ScenarioCallId> {
        match self {
            Self::BindingBefore { call_id, .. } | Self::BindingAfter { call_id, .. } => {
                Some(*call_id)
            }
            _ => None,
        }
    }

    /// Return attempt facts for selector matching.
    pub(crate) const fn attempt(&self) -> Attempt {
        Attempt {
            binding: self.binding_descriptor(),
        }
    }

    /// Return whether this event increments call-scoped trigger counters.
    pub(crate) const fn counts_as_call_event(&self) -> bool {
        matches!(self, Self::BindingBefore { .. })
    }

    /// Return the monotonic timestamp for this event.
    pub(crate) const fn time_ns(&self) -> u64 {
        match self {
            Self::BindingBefore { time_ns, .. }
            | Self::BindingAfter { time_ns, .. }
            | Self::TaskReady { time_ns, .. }
            | Self::TaskStart { time_ns, .. }
            | Self::TimerFire { time_ns, .. }
            | Self::IngressReady { time_ns, .. }
            | Self::ClockRead { time_ns, .. }
            | Self::RandomRead { time_ns, .. } => *time_ns,
        }
    }
}
