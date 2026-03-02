#![allow(clippy::missing_const_for_thread_local)]

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ptr;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformContext, PlatformError,
};
use crate::runtime::bindings::{
    BindingDescriptor, BindingEngine, BindingPolicy, BindingReplayPayload, RuntimeWorld,
};
use crate::runtime::random::RandomStreamId;
use crate::runtime::replay::ReplayController;
use crate::runtime::scheduler::{
    EventLoop, EventLoopScope, MicrotaskId, TaskId, current_event_loop_scope,
};

use super::Agent;
use crate::runtime::{HookState, Hooks};
use destack_workspace::RuntimeAccess;

thread_local! {
    /// TLS slot for the current runtime execution context.
    static CURRENT_AGENT_CONTEXT: Cell<CurrentAgentContext> = const { Cell::new(CurrentAgentContext::empty()) };
    /// TLS slot for the current binding call context.
    static BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> = const { Cell::new(ptr::null()) };
    /// TLS storage for native ABI references returned by bindings.
    static BINDING_CALL_ARENA: BindingCallArena = const { BindingCallArena::new() };
}

/// Current runtime execution context for VM callback bridging.
#[derive(Debug, Clone, Copy)]
struct CurrentAgentContext {
    /// Runtime pointer for callback dispatch.
    runtime: *const Agent,
    /// Event loop pointer for callback dispatch.
    event_loop: *const EventLoop,
}

impl CurrentAgentContext {
    /// Return one empty runtime execution context.
    const fn empty() -> Self {
        Self {
            runtime: ptr::null(),
            event_loop: ptr::null(),
        }
    }

    /// Return whether this execution context is available.
    const fn is_empty(self) -> bool {
        self.runtime.is_null() || self.event_loop.is_null()
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

/// Enter one current-agent execution context for VM callback dispatch.
pub(crate) fn enter_current_agent_context(
    runtime: *const Agent,
    event_loop: *const EventLoop,
) -> CurrentAgentContextGuard {
    let next = CurrentAgentContext {
        runtime,
        event_loop,
    };
    let previous = CURRENT_AGENT_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(next);
        previous
    });

    CurrentAgentContextGuard { previous }
}

/// Return the current-agent execution context when available.
fn current_agent_context() -> Option<CurrentAgentContext> {
    CURRENT_AGENT_CONTEXT.with(|slot| {
        let context = slot.get();
        if context.is_empty() {
            return None;
        }

        Some(context)
    })
}

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct BindingCallContext {
    /// Runtime state for platform bindings.
    runtime: *const Agent,
    /// Event loop for task queues and timers.
    event_loop: *const EventLoop,
    /// Binding policy for external calls.
    policy: Arc<RwLock<BindingPolicy>>,
    /// Engine kind for this binding call.
    engine: BindingEngine,
    /// Event loop scope metadata for the current call.
    scope: EventLoopScope,
}

/// Scope guard that runs after-binding hooks when one binding call completes.
#[derive(Debug)]
pub struct BindingHookGuard<'call> {
    /// Binding call context for hook dispatch.
    context: &'call BindingCallContext,
    /// Binding descriptor for hook dispatch.
    spec: BindingDescriptor,
}

impl Drop for BindingHookGuard<'_> {
    /// Run post-call hooks for this binding call scope.
    fn drop(&mut self) {
        self.context.on_after_binding(self.spec);
    }
}

impl BindingCallContext {
    /// Create a binding call context for TLS.
    pub fn new(runtime: &Agent, event_loop: &EventLoop, policy: BindingPolicy) -> Self {
        Self {
            runtime,
            event_loop,
            policy: Arc::new(RwLock::new(policy)),
            engine: BindingEngine::Native,
            scope: current_event_loop_scope(),
        }
    }

    /// Create a binding call context from raw pointers.
    pub(crate) fn from_raw(
        runtime: *const Agent,
        event_loop: *const EventLoop,
        policy: Arc<RwLock<BindingPolicy>>,
        engine: BindingEngine,
    ) -> Self {
        Self {
            runtime,
            event_loop,
            policy,
            engine,
            scope: current_event_loop_scope(),
        }
    }

    /// Create one VM binding call context from the current-agent execution scope.
    pub(crate) fn from_current_agent_for_vm(
        policy: Arc<RwLock<BindingPolicy>>,
    ) -> RuntimeResult<Self> {
        let context = current_agent_context()
            .ok_or_else(|| RuntimeError::BindingCallContextMissing.boxed())?;
        Ok(Self::from_raw(
            context.runtime,
            context.event_loop,
            policy,
            BindingEngine::Vm,
        ))
    }

    /// Borrow the runtime state.
    #[inline]
    pub fn runtime(&self) -> &Agent {
        // safety: pointer is owned by the runtime caller
        unsafe { &*self.runtime }
    }

    /// Borrow the event loop.
    #[inline]
    pub fn event_loop(&self) -> &EventLoop {
        // safety: pointer is owned by the runtime
        unsafe { &*self.event_loop }
    }

    /// Borrow the platform context.
    #[inline]
    pub fn platform(&self) -> &PlatformContext {
        &self.runtime().platform
    }

    /// Borrow the replay state.
    #[inline]
    pub fn replay(&self) -> &ReplayController {
        self.world().replay()
    }

    /// Borrow the runtime hook state.
    #[inline]
    pub fn hooks(&self) -> &Hooks {
        self.runtime().hooks.as_ref()
    }

    /// Borrow the runtime host state.
    #[inline]
    pub fn host(&self) -> &crate::host::Host {
        &self.runtime().host
    }

    /// Borrow the shared runtime world.
    #[inline]
    pub fn world(&self) -> &crate::runtime::world::World {
        self.runtime().world.as_ref()
    }

    /// Borrow one read guard for the simulation state.
    #[inline]
    pub fn read_simulation(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, crate::simulation::Simulation> {
        self.world().read_simulation()
    }

    /// Borrow one write guard for the simulation state.
    #[inline]
    pub fn write_simulation(
        &self,
    ) -> parking_lot::RwLockWriteGuard<'_, crate::simulation::Simulation> {
        self.world().write_simulation()
    }

    /// Return the current event loop scope.
    pub const fn scope(&self) -> EventLoopScope {
        self.scope
    }

    /// Return the current task identifier.
    pub const fn task_id(&self) -> Option<TaskId> {
        self.scope.task_id()
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(&self) -> Option<MicrotaskId> {
        self.scope.microtask_id()
    }

    /// Return the current random stream identifier.
    pub const fn random_stream_id(&self) -> RandomStreamId {
        self.scope.random_stream_id()
    }

    /// Return the engine kind for this call context.
    pub const fn engine(&self) -> BindingEngine {
        self.engine
    }

    /// Clear call-local storage for native bindings.
    pub fn clear_values(&self) {
        BINDING_CALL_ARENA.with(|arena| arena.clear());
    }

    /// Store a string for the duration of the current call.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        BINDING_CALL_ARENA.with(|arena| arena.store_string(value))
    }

    /// Store an optional string for the duration of the current call.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        BINDING_CALL_ARENA.with(|arena| arena.store_string_option(value))
    }

    /// Store a slice for the duration of the current call.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        BINDING_CALL_ARENA.with(|arena| arena.store_slice(values))
    }

    /// Store an array for the duration of the current call.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        BINDING_CALL_ARENA.with(|arena| arena.store_array(values))
    }

    /// Store a string slice for the duration of the current call.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        BINDING_CALL_ARENA.with(|arena| arena.store_string_slice(values))
    }

    /// Return one policy-violation error for one binding descriptor.
    fn policy_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::PolicyViolation {
            name: spec.name.to_string(),
        }
    }

    /// Ensure world access routing allows this binding call.
    fn ensure_binding_access_allowed(
        &self,
        spec: BindingDescriptor,
        policy: &BindingPolicy,
    ) -> RuntimeResult<()> {
        let access = self.world().resolve_binding_access(
            self.hooks().execution_mode(),
            self.hooks().policy_identity(),
            spec,
            Some(self.engine),
            policy.default_access(),
        );
        if access == RuntimeAccess::Deny {
            return Err(self.policy_violation_error(spec).boxed());
        }

        Ok(())
    }

    /// Run pre-call binding policy and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingHookGuard<'_>> {
        // run rule hooks and policy checks first
        self.hooks()
            .on_before_binding(spec, HookState::from_engine(Some(self.engine)))?;
        let policy = self.policy.read();
        policy.ensure_allowed_for_engine(spec, Some(self.engine))?;
        self.ensure_binding_access_allowed(spec, &policy)?;

        Ok(BindingHookGuard {
            context: self,
            spec,
        })
    }

    /// Run pre-call policy, resolve world, and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding_resolve_world(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<(RuntimeWorld, BindingHookGuard<'_>)> {
        // run rule hooks and policy checks first
        self.hooks()
            .on_before_binding(spec, HookState::from_engine(Some(self.engine)))?;
        let policy = self.policy.read();
        policy.ensure_allowed_for_engine(spec, Some(self.engine))?;
        self.ensure_binding_access_allowed(spec, &policy)?;

        // apply world-scoped world routing rules
        let world = self.world().resolve_binding_world(
            self.hooks().execution_mode(),
            self.hooks().policy_identity(),
            spec,
            Some(self.engine),
            policy.default_world(),
        );

        // reject host dispatch when the binding is unavailable on this host
        if world == RuntimeWorld::Host && !spec.supports_current_host() {
            return Err(RuntimeError::from(PlatformError::not_supported(spec.name)).boxed());
        }

        let hook_guard = BindingHookGuard {
            context: self,
            spec,
        };
        Ok((world, hook_guard))
    }

    /// Run post-call hooks for one binding descriptor.
    #[inline]
    pub fn on_after_binding(&self, spec: BindingDescriptor) {
        self.hooks()
            .on_after_binding(spec, HookState::from_engine(Some(self.engine)));
    }

    /// Resolve the binding world for this call context.
    #[inline]
    pub fn resolve_world(&self, spec: BindingDescriptor) -> RuntimeWorld {
        let policy = self.policy.read();
        self.world().resolve_binding_world(
            self.hooks().execution_mode(),
            self.hooks().policy_identity(),
            spec,
            Some(self.engine),
            policy.default_world(),
        )
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let policy = self.policy.read();
        let requested = self.world().resolve_binding_replay_payload(
            self.hooks().execution_mode(),
            self.hooks().policy_identity(),
            spec,
            Some(self.engine),
            policy.default_replay_payload(),
        );
        self.replay().payload_policy_for_requested(spec, requested)
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
        BINDING_CALL_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Enter a binding call context for native bindings.
#[inline]
pub fn enter_binding_call_context(context: &BindingCallContext) -> BindingCallGuard {
    // swap in the new TLS context and capture the previous one
    let previous = BINDING_CALL_CONTEXT.with(|slot| {
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
    // read the TLS context pointer
    let context = BINDING_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::BindingCallContextMissing.boxed());
    }

    // safety: pointer is set by enter_binding_call_context
    let context = unsafe { &*context };

    // clear call-local value storage
    context.clear_values();
    f(context)
}

/// Per-call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same thread.
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
