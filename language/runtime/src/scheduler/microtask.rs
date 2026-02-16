use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::runnable::PlatformRunnable;
use super::task::TaskState;

/// Opaque microtask identifier used by the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MicrotaskId(u64);

impl MicrotaskId {
    /// Create a new microtask identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw microtask identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Scheduler microtask metadata for Promise jobs.
#[derive(Debug)]
pub struct Microtask {
    /// Microtask identifier used for ordering and logging.
    pub id: MicrotaskId,
    /// Runnable continuation for this microtask.
    pub runnable: PlatformRunnable,
    /// Resume payload passed back into the executor.
    pub resume_value: vm::Value,
    /// Current scheduling state.
    pub state: TaskState,
}
