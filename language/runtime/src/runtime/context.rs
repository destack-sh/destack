use std::cell::Cell;
use std::ptr;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeErrorStore, RuntimeResult};
use crate::platform::bindings::{BindingDescriptor, BindingPolicy, ExecutionMode};
use crate::platform::{NativeStringRef, PlatformContext, ResourceTable};
use crate::random::Random;
use crate::replay::{ReplayHeader, ReplayLogState};
use crate::runtime::RuntimeCallStringStore;
use crate::scheduler::Scheduler;
use crate::telemetry::Telemetry;
use crate::time::Clock;

thread_local! {
    /// TLS slot for the current runtime call context.
    static RUNTIME_CALL_CONTEXT: Cell<*const RuntimeCallContext> = const { Cell::new(ptr::null()) };
    /// TLS storage for native string references returned by bindings.
    static RUNTIME_CALL_STRINGS: RuntimeCallStringStore = RuntimeCallStringStore::default();
}

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeState {
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Virtual time and clock policy.
    pub time: Clock,
    /// Deterministic randomness streams.
    pub random: Random,
    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Replay log and record/replay state.
    pub replay: ReplayLogState,
    /// Runtime error storage for native bindings.
    pub errors: RuntimeErrorStore,
    /// Telemetry aggregation for debugging and profiling.
    pub telemetry: Telemetry,
}

/// Cloneable handle to runtime state.
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Shared runtime state.
    state: Arc<RuntimeState>,
}

impl RuntimeContext {
    /// Create a runtime context from explicit platform state.
    pub fn new(platform: PlatformContext) -> Self {
        Self::with_execution_mode(platform, ExecutionMode::Fast)
    }

    /// Create a runtime context with an explicit execution mode.
    pub fn with_execution_mode(platform: PlatformContext, mode: ExecutionMode) -> Self {
        Self::with_replay_header(platform, mode, ReplayHeader::default())
    }

    /// Create a runtime context with an explicit replay header.
    pub fn with_replay_header(
        platform: PlatformContext,
        mode: ExecutionMode,
        header: ReplayHeader,
    ) -> Self {
        Self {
            state: Arc::new(RuntimeState {
                platform,
                time: Clock::default(),
                random: Random::default(),
                resources: ResourceTable::default(),
                replay: ReplayLogState::new(mode, header),
                errors: RuntimeErrorStore::default(),
                telemetry: Telemetry,
            }),
        }
    }

    /// Return the platform context.
    pub fn platform(&self) -> &PlatformContext {
        &self.state.platform
    }

    /// Return the runtime clock.
    pub fn time(&self) -> &Clock {
        &self.state.time
    }

    /// Return the runtime random source.
    pub fn random(&self) -> &Random {
        &self.state.random
    }

    /// Return the resource table.
    pub fn resources(&self) -> &ResourceTable {
        &self.state.resources
    }

    /// Return the replay log.
    pub fn replay(&self) -> &ReplayLogState {
        &self.state.replay
    }

    /// Return the error store.
    pub fn errors(&self) -> &RuntimeErrorStore {
        &self.state.errors
    }

    /// Return telemetry aggregation.
    pub fn telemetry(&self) -> &Telemetry {
        &self.state.telemetry
    }

    /// Return a raw pointer to the runtime state for internal use.
    pub(crate) fn state_ptr(&self) -> *const RuntimeState {
        Arc::as_ptr(&self.state)
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new(PlatformContext::new(Vec::new()))
    }
}

/// TLS payload for native runtime calls.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeCallContext {
    /// Runtime state for platform bindings.
    runtime: *const RuntimeState,
    /// Scheduler for task queues and timers.
    scheduler: *const Scheduler,
    /// Binding policy for external calls.
    policy: BindingPolicy,
}

impl RuntimeCallContext {
    /// Create a runtime call context for TLS.
    pub fn new(runtime: &RuntimeContext, scheduler: &Scheduler, policy: BindingPolicy) -> Self {
        Self {
            runtime: Arc::as_ptr(&runtime.state),
            scheduler,
            policy,
        }
    }

    /// Create a runtime call context from raw pointers.
    pub(crate) fn from_raw(
        runtime: *const RuntimeState,
        scheduler: *const Scheduler,
        policy: BindingPolicy,
    ) -> Self {
        Self {
            runtime,
            scheduler,
            policy,
        }
    }

    /// Borrow the runtime state.
    #[inline]
    pub fn runtime(&self) -> &RuntimeState {
        // safety: pointer is owned by an Arc in the caller
        unsafe { &*self.runtime }
    }

    /// Borrow the scheduler.
    #[inline]
    pub fn scheduler(&self) -> &Scheduler {
        // safety: pointer is owned by the runtime
        unsafe { &*self.scheduler }
    }

    /// Borrow the platform context.
    #[inline]
    pub fn platform(&self) -> &PlatformContext {
        &self.runtime().platform
    }

    /// Borrow the replay state.
    #[inline]
    pub fn replay(&self) -> &ReplayLogState {
        &self.runtime().replay
    }

    /// Clear call-local string storage.
    pub fn clear_strings(&self) {
        RUNTIME_CALL_STRINGS.with(|store| store.clear());
    }

    /// Store a string for the duration of the current call.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        RUNTIME_CALL_STRINGS.with(|store| store.store(value))
    }

    /// Store an optional string for the duration of the current call.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        RUNTIME_CALL_STRINGS.with(|store| store.store_option(value))
    }

    /// Validate the policy against a binding descriptor.
    #[inline]
    pub fn check_policy(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        self.policy.check(spec)
    }
}

/// Guard that restores the previous TLS runtime call context.
#[derive(Debug)]
pub struct RuntimeCallGuard {
    /// Previous TLS context pointer.
    previous: *const RuntimeCallContext,
}

impl Drop for RuntimeCallGuard {
    /// Restore the previous runtime call context.
    fn drop(&mut self) {
        RUNTIME_CALL_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Enter a runtime call context for native bindings.
#[inline]
pub fn enter_runtime_call_context(context: &RuntimeCallContext) -> RuntimeCallGuard {
    let previous = RUNTIME_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const RuntimeCallContext);
        previous
    });

    RuntimeCallGuard { previous }
}

/// Access the current runtime call context for native bindings.
#[inline]
pub fn with_runtime_call_context<T>(
    f: impl FnOnce(&RuntimeCallContext) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let context = RUNTIME_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::runtime_context_missing().boxed());
    }

    // safety: pointer is set by enter_runtime_call_context
    let context = unsafe { &*context };
    context.clear_strings();
    f(context)
}
