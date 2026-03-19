#![allow(clippy::missing_const_for_thread_local)]

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ptr;

use serde::{Deserialize, Serialize};

use super::agent::Agent;
use super::call::BindingCallContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostSession;
use crate::platform::NativeArray;
use crate::runtime::bindings::BindingAffinity;
use crate::runtime::scheduler::{EventLoop, MicrotaskId, TaskId};
use crate::runtime::world::World;
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

thread_local! {
    /// TLS slot for the current runtime execution context.
    static CURRENT_AGENT_CONTEXT: Cell<CurrentAgentContext> =
        const { Cell::new(CurrentAgentContext::empty()) };
    /// TLS slot for the current binding call context.
    static CURRENT_BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> =
        const { Cell::new(ptr::null()) };
    /// TLS storage for native ABI references returned by bindings.
    static CURRENT_BINDING_CALL_ARENA: BindingCallArena = const { BindingCallArena::new() };
    /// TLS slot for the current event loop scope.
    static CURRENT_EVENT_LOOP_SCOPE: Cell<EventLoopScope> =
        const { Cell::new(EventLoopScope::empty()) };
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
pub(crate) struct CurrentAgentContext {
    /// Agent pointer for callback dispatch.
    pub agent: *const Agent,
    /// Event loop pointer for callback dispatch.
    pub event_loop: *const EventLoop,
    /// Host pointer for callback dispatch.
    pub host: *const HostSession,
    /// World pointer for replay, time, random, and policy.
    pub world: *const World,
    /// Execution context identifier for callback dispatch.
    pub execution_context_id: ExecutionContextId,
    /// Whether this execution scope runs on the process main context.
    pub is_process_main: bool,
}

impl CurrentAgentContext {
    /// Return one empty runtime execution context.
    pub(crate) const fn empty() -> Self {
        Self {
            agent: ptr::null(),
            event_loop: ptr::null(),
            host: ptr::null(),
            world: ptr::null(),
            execution_context_id: ExecutionContextId(0),
            is_process_main: false,
        }
    }

    /// Return whether this execution context is available.
    pub(crate) const fn is_empty(self) -> bool {
        self.agent.is_null()
            || self.event_loop.is_null()
            || self.host.is_null()
            || self.world.is_null()
    }
}

/// Guard that restores the previous current-agent execution context.
#[derive(Debug)]
pub(crate) struct CurrentAgentContextGuard {
    /// Previous current-agent execution context.
    previous: CurrentAgentContext,
}

impl Drop for CurrentAgentContextGuard {
    /// Restore the previous current-agent execution context.
    fn drop(&mut self) {
        CURRENT_AGENT_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Event loop scope for runtime execution.
#[derive(Debug, Clone, Copy)]
pub struct EventLoopScope {
    /// Current task identifier, if any.
    task_id: Option<TaskId>,
    /// Current microtask identifier, if any.
    microtask_id: Option<MicrotaskId>,
    /// Current nested microtask execution depth.
    microtask_depth: usize,
}

impl EventLoopScope {
    /// Create an empty event loop scope.
    pub const fn empty() -> Self {
        Self {
            task_id: None,
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a task event loop scope.
    pub const fn for_task(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a microtask event loop scope.
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

/// Guard that restores the previous event loop scope.
#[derive(Debug)]
pub struct EventLoopScopeGuard {
    /// Previous event loop scope.
    previous: EventLoopScope,
}

impl Drop for EventLoopScopeGuard {
    /// Restore the previous event loop scope.
    fn drop(&mut self) {
        CURRENT_EVENT_LOOP_SCOPE.with(|slot| slot.set(self.previous));
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

/// Per-call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same native thread.
#[derive(Debug, Default)]
pub struct BindingCallArena {
    /// Owned strings backing native string references.
    strings: RefCell<Vec<Box<str>>>,
    /// Owned slices backing native slice references.
    values: RefCell<Vec<Box<dyn Any>>>,
}

impl BindingCallArena {
    /// Create an empty call arena.
    pub const fn new() -> Self {
        Self {
            strings: RefCell::new(Vec::new()),
            values: RefCell::new(Vec::new()),
        }
    }

    /// Clear all stored references.
    pub fn clear(&self) {
        self.strings.borrow_mut().clear();
        self.values.borrow_mut().clear();
    }

    /// Store a string and return a native string reference.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        let mut strings = self.strings.borrow_mut();
        strings.push(value.to_owned().into_boxed_str());

        let stored = strings.last().expect("stored string must be available");
        NativeStringRef::from(stored.as_ref())
    }

    /// Store an optional string and return a native string reference.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        match value {
            Some(value) => self.store_string(value),
            None => NativeStringRef {
                data: ptr::null(),
                len: 0,
            },
        }
    }

    /// Store a slice and return a native slice reference.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeSlice { data, len }
    }

    /// Store a slice and return a native array reference.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeArray {
            data,
            len,
            capacity: len,
        }
    }

    /// Store a string slice and return a native string slice.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr() as *const NativeStringRef;
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeStringSlice { data, len }
    }
}

/// Return the stable metadata name for one binding-affinity class.
pub const fn binding_affinity_name(affinity: BindingAffinity) -> &'static str {
    match affinity {
        BindingAffinity::Any => "any",
        BindingAffinity::EventLoop => "eventLoop",
        BindingAffinity::Owner => "owner",
        BindingAffinity::ProcessMain => "processMain",
    }
}

/// Enter one current-agent execution context for VM callback dispatch.
pub(crate) fn enter_current_agent_context(
    agent: *const Agent,
    event_loop: *const EventLoop,
    host: *const HostSession,
    world: *const World,
    is_process_main: bool,
) -> CurrentAgentContextGuard {
    let event_loop = unsafe { &*event_loop };
    let execution_context_id = event_loop.execution_context_id();
    let next = CurrentAgentContext {
        agent,
        event_loop: event_loop as *const EventLoop,
        host,
        world,
        execution_context_id,
        is_process_main,
    };
    let previous = CURRENT_AGENT_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(next);
        previous
    });

    CurrentAgentContextGuard { previous }
}

/// Return the current-agent execution context when available.
pub(crate) fn current_agent_context() -> Option<CurrentAgentContext> {
    CURRENT_AGENT_CONTEXT.with(|slot| {
        let context = slot.get();
        if context.is_empty() {
            return None;
        }

        Some(context)
    })
}

/// Enter an event loop scope for runtime execution.
#[inline]
pub(crate) fn enter_event_loop_scope(scope: EventLoopScope) -> EventLoopScopeGuard {
    let previous = CURRENT_EVENT_LOOP_SCOPE.with(|slot| {
        let previous = slot.get();
        slot.set(scope);
        previous
    });

    EventLoopScopeGuard { previous }
}

/// Return the current event loop scope.
#[inline]
pub(crate) fn current_event_loop_scope() -> EventLoopScope {
    CURRENT_EVENT_LOOP_SCOPE.with(|slot| slot.get())
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

/// Borrow the current binding call arena.
pub(crate) fn with_binding_call_arena<T>(f: impl FnOnce(&BindingCallArena) -> T) -> T {
    CURRENT_BINDING_CALL_ARENA.with(f)
}
