use std::cell::Cell;

use serde::{Deserialize, Serialize};

use crate::host::binding::BindingAffinity;
use crate::runtime::scheduler::RunnableId;

thread_local! {
    /// TLS slot for the currently running task or microtask.
    static CURRENT_RUNNABLE_SCOPE: Cell<RunnableScope> =
        const { Cell::new(RunnableScope::empty()) };
}

/// Currently running task or microtask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnableScope {
    /// No task or microtask is active.
    Empty,
    /// One task is active.
    Task {
        /// Active task identifier.
        task_id: RunnableId,
    },
    /// One microtask is active.
    Microtask {
        /// Active microtask identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },
}

/// Runtime work that made visible scheduler progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RunnableProgress {
    /// One task progressed.
    Task {
        /// Task that progressed.
        task_id: RunnableId,
    },
    /// One microtask progressed.
    Microtask {
        /// Microtask that progressed.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },
}

impl RunnableScope {
    /// Create an empty runnable scope.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Create a task runnable scope.
    pub const fn for_task(task_id: RunnableId) -> Self {
        Self::Task { task_id }
    }

    /// Create a microtask runnable scope.
    pub fn for_microtask(microtask_id: RunnableId, depth: usize) -> Self {
        debug_assert!(u32::try_from(depth).is_ok());
        let depth = depth as u32;

        Self::Microtask {
            microtask_id,
            depth,
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(self) -> Option<RunnableId> {
        match self {
            Self::Task { task_id } => Some(task_id),
            Self::Empty | Self::Microtask { .. } => None,
        }
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(self) -> Option<RunnableId> {
        match self {
            Self::Microtask { microtask_id, .. } => Some(microtask_id),
            Self::Empty | Self::Task { .. } => None,
        }
    }

    /// Return the current microtask nesting depth.
    pub const fn microtask_depth(self) -> usize {
        match self {
            Self::Microtask { depth, .. } => depth as usize,
            Self::Empty | Self::Task { .. } => 0,
        }
    }

    /// Return the progress represented by one active runnable scope.
    pub(crate) const fn progress(self) -> Option<RunnableProgress> {
        match self {
            Self::Task { task_id } => Some(RunnableProgress::task(task_id)),
            Self::Microtask {
                microtask_id,
                depth,
            } => Some(RunnableProgress::microtask(microtask_id, depth)),
            Self::Empty => None,
        }
    }
}

impl RunnableProgress {
    /// Create one task progress value.
    pub(crate) const fn task(task_id: RunnableId) -> Self {
        Self::Task { task_id }
    }

    /// Create one microtask progress value.
    pub(crate) const fn microtask(microtask_id: RunnableId, depth: u32) -> Self {
        Self::Microtask {
            microtask_id,
            depth,
        }
    }
}

/// Guard that restores the previous runnable scope.
#[derive(Debug)]
pub struct RunnableScopeGuard {
    /// Previous runnable scope.
    previous: RunnableScope,
}

impl Drop for RunnableScopeGuard {
    /// Restore the previous runnable scope.
    fn drop(&mut self) {
        CURRENT_RUNNABLE_SCOPE.with(|slot| slot.set(self.previous));
    }
}

/// Return the stable metadata name for one binding-affinity class.
pub const fn binding_affinity_name(affinity: BindingAffinity) -> &'static str {
    match affinity {
        BindingAffinity::None => "none",
        BindingAffinity::Worker => "worker",
        BindingAffinity::Main => "main",
    }
}

/// Enter the scope for the currently running task or microtask.
#[inline]
pub(crate) fn enter_runnable_scope(scope: RunnableScope) -> RunnableScopeGuard {
    let previous = CURRENT_RUNNABLE_SCOPE.with(|slot| {
        let previous = slot.get();
        slot.set(scope);
        previous
    });

    RunnableScopeGuard { previous }
}

/// Return the currently running task or microtask.
#[inline]
pub(crate) fn current_runnable_scope() -> RunnableScope {
    CURRENT_RUNNABLE_SCOPE.with(|slot| slot.get())
}
