use std::cell::Cell;
use std::ptr;
use std::sync::Arc;

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

use super::{BindingCallArena, RuntimeState};
use crate::runtime::{RuntimeHookState, RuntimeRules};

thread_local! {
    /// TLS slot for the current binding call context.
    static BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> = const { Cell::new(ptr::null()) };
    /// TLS storage for native ABI references returned by bindings.
    static BINDING_CALL_ARENA: BindingCallArena = BindingCallArena::default();
}

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct BindingCallContext {
    /// Runtime state for platform bindings.
    runtime: *const RuntimeState,
    /// Event loop for task queues and timers.
    event_loop: *const EventLoop,
    /// Binding policy for external calls.
    policy: Arc<BindingPolicy>,
    /// Engine kind for this binding call.
    engine: BindingEngine,
    /// Event loop scope metadata for the current call.
    scope: EventLoopScope,
}

impl BindingCallContext {
    /// Create a binding call context for TLS.
    pub fn new(runtime: &Arc<RuntimeState>, event_loop: &EventLoop, policy: BindingPolicy) -> Self {
        Self {
            runtime: Arc::as_ptr(runtime),
            event_loop,
            policy: Arc::new(policy),
            engine: BindingEngine::Native,
            scope: current_event_loop_scope(),
        }
    }

    /// Create a binding call context from raw pointers.
    pub(crate) fn from_raw(
        runtime: *const RuntimeState,
        event_loop: *const EventLoop,
        policy: Arc<BindingPolicy>,
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

    /// Borrow the runtime state.
    #[inline]
    pub fn runtime(&self) -> &RuntimeState {
        // safety: pointer is owned by an Arc in the caller
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
        &self.runtime().replay
    }

    /// Borrow the runtime rule state.
    #[inline]
    pub fn rules(&self) -> &RuntimeRules {
        &self.runtime().rules
    }

    /// Borrow the shared simulation state.
    #[inline]
    pub fn simulation(&self) -> &crate::simulation::SharedSimulationState {
        &self.runtime().simulation
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

    /// Validate the policy against a binding descriptor.
    #[inline]
    pub fn check_policy(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        let _ = self.check_and_resolve_world(spec)?;
        Ok(())
    }

    /// Validate policy and resolve the binding world for this call context.
    #[inline]
    pub fn check_and_resolve_world(&self, spec: BindingDescriptor) -> RuntimeResult<RuntimeWorld> {
        // run rule hooks and policy checks first
        self.rules()
            .on_before_binding(spec, RuntimeHookState::from_engine(Some(self.engine)))?;
        let world = self
            .policy
            .check_and_resolve_world_for_engine(spec, Some(self.engine))?;

        // reject host dispatch when the binding is unavailable on this host
        if world == RuntimeWorld::Host && !spec.supports_current_host() {
            return Err(RuntimeError::from(PlatformError::not_supported(spec.name)).boxed());
        }

        Ok(world)
    }

    /// Resolve the binding world for this call context.
    #[inline]
    pub fn resolve_world(&self, spec: BindingDescriptor) -> RuntimeWorld {
        self.policy
            .resolve_world_for_engine(spec, Some(self.engine))
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let requested = self
            .policy
            .resolve_replay_payload_for_engine(spec, Some(self.engine));
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

/// Run a closure with the current TLS binding call context when present.
#[inline]
pub(crate) fn with_current_binding_call_context<T>(
    f: impl FnOnce(&BindingCallContext) -> T,
) -> Option<T> {
    // read the TLS context pointer
    let context = BINDING_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return None;
    }

    // safety: pointer is set by enter_binding_call_context
    let context = unsafe { &*context };
    Some(f(context))
}
