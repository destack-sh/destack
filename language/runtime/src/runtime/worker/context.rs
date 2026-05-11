use std::cell::Cell;
use std::ptr;

use serde::{Deserialize, Serialize};

use super::call::BindingCallContext;
use super::worker::Worker;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Session;
use crate::runtime::binding::BindingAffinity;
use crate::runtime::scheduler::{EventLoop, MicrotaskId, TaskId};
use crate::runtime::world::WorldState;

thread_local! {
    /// TLS slot for the current runtime execution context.
    static CURRENT_WORKER_CONTEXT: Cell<CurrentWorkerContext> =
        const { Cell::new(CurrentWorkerContext::empty()) };
    /// TLS slot for the current binding call context.
    static CURRENT_BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> =
        const { Cell::new(ptr::null()) };
    /// TLS slot for the currently running task or microtask.
    static CURRENT_RUNNABLE_SCOPE: Cell<RunnableScope> =
        const { Cell::new(RunnableScope::empty()) };
}

/// Stable identifier for one execution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionContextId(pub u64);

impl ExecutionContextId {
    /// Build one identifier from an opaque hash payload.
    pub const fn from_hash(hash: u64) -> Self {
        Self(hash)
    }
}

/// Runtime execution context for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Stable execution context identifier.
    pub id: ExecutionContextId,
    /// Whether the current execution context is the process main context.
    pub is_process_main: bool,
}

impl ExecutionContext {
    /// Build one execution context payload.
    pub const fn new(id: ExecutionContextId, is_process_main: bool) -> Self {
        Self {
            id,
            is_process_main,
        }
    }
}

/// Current runtime execution context for VM callback bridging.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CurrentWorkerContext {
    /// Worker pointer for VM callbacks.
    pub worker: *mut Worker,
    /// Event loop pointer for VM callbacks.
    pub event_loop: *const EventLoop,
    /// Host pointer for VM callbacks.
    pub host: *const Session,
    /// World pointer for replay, time, random, and policy.
    pub world: *mut WorldState,
    /// Execution context identifier for VM callbacks.
    pub execution_context_id: ExecutionContextId,
    /// Whether this execution scope runs on the process main context.
    pub is_process_main: bool,
}

impl CurrentWorkerContext {
    /// Return one empty runtime execution context.
    pub(crate) const fn empty() -> Self {
        Self {
            worker: ptr::null_mut(),
            event_loop: ptr::null(),
            host: ptr::null(),
            world: ptr::null_mut(),
            execution_context_id: ExecutionContextId(0),
            is_process_main: false,
        }
    }

    /// Return whether this execution context is available.
    pub(crate) const fn is_empty(self) -> bool {
        self.worker.is_null()
            || self.event_loop.is_null()
            || self.host.is_null()
            || self.world.is_null()
    }
}

/// Guard that restores the previous current-worker execution context.
#[derive(Debug)]
pub(crate) struct CurrentWorkerContextGuard {
    /// Previous current-worker execution context.
    previous: CurrentWorkerContext,
}

impl Drop for CurrentWorkerContextGuard {
    /// Restore the previous current-worker execution context.
    fn drop(&mut self) {
        CURRENT_WORKER_CONTEXT.with(|slot| slot.set(self.previous));
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

/// Guard that restores the previous TLS binding call context.
#[derive(Debug)]
pub struct BindingCallGuard {
    /// Previous TLS context pointer.
    previous: *const BindingCallContext,
}

impl Drop for BindingCallGuard {
    /// Restore the previous binding call context.
    fn drop(&mut self) {
        CURRENT_BINDING_CALL_CONTEXT.with(|slot| slot.set(self.previous));
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

/// Enter one current-worker execution context for VM callbacks.
pub(crate) fn enter_current_worker_context(
    worker: *mut Worker,
    event_loop: *const EventLoop,
    host: *const Session,
    world: *mut WorldState,
    is_process_main: bool,
) -> CurrentWorkerContextGuard {
    let event_loop = unsafe { &*event_loop };
    let execution_context_id = event_loop.execution_context_id();
    let next = CurrentWorkerContext {
        worker,
        event_loop: event_loop as *const EventLoop,
        host,
        world,
        execution_context_id,
        is_process_main,
    };
    let previous = CURRENT_WORKER_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(next);
        previous
    });

    CurrentWorkerContextGuard { previous }
}

/// Return the current-worker execution context when available.
pub(crate) fn current_worker_context() -> Option<CurrentWorkerContext> {
    CURRENT_WORKER_CONTEXT.with(|slot| {
        let context = slot.get();
        if context.is_empty() {
            return None;
        }

        Some(context)
    })
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

/// Enter a binding call context for native bindings.
#[inline]
pub fn enter_binding_call_context(context: &BindingCallContext) -> BindingCallGuard {
    let previous = CURRENT_BINDING_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const BindingCallContext);
        previous
    });

    BindingCallGuard { previous }
}

/// Access the current binding call context for native bindings.
#[inline]
pub fn with_binding_call_context<T>(
    f: impl FnOnce(&BindingCallContext) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let context = CURRENT_BINDING_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::BindingCallContextMissing.boxed());
    }

    let context = unsafe { &*context };
    context.clear_values();
    f(context)
}
