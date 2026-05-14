use std::cell::Cell;

use serde::{Deserialize, Serialize};

use crate::host::binding::BindingAffinity;
use crate::runtime::scheduler::{MicrotaskId, TaskId};

thread_local! {
    /// TLS slot for the currently running task or microtask.
    static CURRENT_RUNNABLE_SCOPE: Cell<RunnableScope> =
        const { Cell::new(RunnableScope::empty()) };
}

/// Runtime execution context for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Whether the current execution context is the process main context.
    pub is_process_main: bool,
}

impl ExecutionContext {
    /// Build one execution context payload.
    pub const fn new(is_process_main: bool) -> Self {
        Self { is_process_main }
    }
}

/// Currently running task or microtask.
#[derive(Debug, Clone, Copy)]
pub struct RunnableScope {
    /// Current task identifier, if any.
    task_id: Option<TaskId>,
    /// Current microtask identifier, if any.
    microtask_id: Option<MicrotaskId>,
    /// Current nested microtask execution depth.
    microtask_depth: usize,
}

impl RunnableScope {
    /// Create an empty runnable scope.
    pub const fn empty() -> Self {
        Self {
            task_id: None,
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a task runnable scope.
    pub const fn for_task(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a microtask runnable scope.
    pub const fn for_microtask(microtask_id: MicrotaskId, depth: usize) -> Self {
        Self {
            task_id: None,
            microtask_id: Some(microtask_id),
            microtask_depth: depth,
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(self) -> Option<TaskId> {
        self.task_id
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(self) -> Option<MicrotaskId> {
        self.microtask_id
    }

    /// Return the current microtask nesting depth.
    pub const fn microtask_depth(self) -> usize {
        self.microtask_depth
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
