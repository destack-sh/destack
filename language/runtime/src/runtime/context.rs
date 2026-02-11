use std::cell::Cell;
use std::ptr;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeErrorStore, RuntimeResult};
use crate::platform::bindings::{
    BindingDescriptor, BindingPolicy, ExecutionMode, PolicyEngine, ReplayPayload,
};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformContext, ResourceTable,
};
use crate::random::{Random, RandomStreamId};
use crate::replay::{ReplayController, ReplayHeader};
use crate::runtime::{RuntimeCallStringStore, RuntimeCallValueStore};
use crate::scheduler::{MicrotaskId, Scheduler, TaskId};
use crate::time::Clock;
use destack_workspace::{
    ExecutionMode as WorkspaceExecutionMode, GcOptions, PlatformOptions, PlatformWindowsOptions,
    RandomMode, RandomOptions, ReplayLogOptions, ReplayPayloadMode, RuntimeOptions, TimeMode,
    TimeOptions,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

thread_local! {
    /// TLS slot for the current runtime call context.
    static RUNTIME_CALL_CONTEXT: Cell<*const RuntimeCallContext> = const { Cell::new(ptr::null()) };
    /// TLS slot for the current execution context.
    static RUNTIME_EXECUTION_CONTEXT: Cell<ExecutionContext> =
        const { Cell::new(ExecutionContext::empty()) };
    /// TLS storage for native string references returned by bindings.
    static RUNTIME_CALL_STRINGS: RuntimeCallStringStore = RuntimeCallStringStore::default();
    static RUNTIME_CALL_VALUES: RuntimeCallValueStore = RuntimeCallValueStore::default();
}

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeState {
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Platform runtime configuration options.
    pub platform_options: PlatformOptions,
    /// Virtual time and clock policy.
    pub time: Clock,
    /// Deterministic randomness streams.
    pub random: Random,
    /// Runtime GC options for heap policy.
    pub gc_options: GcOptions,
    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Replay log and record/replay state.
    pub replay: ReplayController,
    /// Runtime error storage for native bindings.
    pub errors: RuntimeErrorStore,
}

/// Execution context for runtime scheduling.
#[derive(Debug, Clone, Copy)]
pub struct ExecutionContext {
    /// Current task identifier, if any.
    task_id: Option<TaskId>,
    /// Current microtask identifier, if any.
    microtask_id: Option<MicrotaskId>,
}

impl ExecutionContext {
    /// Create an empty execution context.
    pub const fn empty() -> Self {
        Self {
            task_id: None,
            microtask_id: None,
        }
    }

    /// Create a task execution context.
    pub const fn for_task(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            microtask_id: None,
        }
    }

    /// Create a microtask execution context.
    pub const fn for_microtask(microtask_id: MicrotaskId) -> Self {
        Self {
            task_id: None,
            microtask_id: Some(microtask_id),
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

    /// Return the random stream identifier for this context.
    pub const fn random_stream_id(self) -> RandomStreamId {
        // use tagged stream ids to avoid collisions between scheduler sources
        const STREAM_ID_MASK: u64 = (1u64 << 62) - 1;
        const TASK_TAG: u64 = 1u64 << 62;
        const MICROTASK_TAG: u64 = 2u64 << 62;
        match (self.task_id, self.microtask_id) {
            (_, Some(microtask_id)) => {
                RandomStreamId::new(MICROTASK_TAG | (microtask_id.get() & STREAM_ID_MASK))
            }
            (Some(task_id), None) => {
                RandomStreamId::new(TASK_TAG | (task_id.get() & STREAM_ID_MASK))
            }
            (None, None) => RandomStreamId::DEFAULT,
        }
    }
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
        Self::from_runtime_options(platform, &RuntimeOptions::default())
    }

    /// Create a runtime context with an explicit execution mode.
    pub fn from_execution_mode(platform: PlatformContext, mode: ExecutionMode) -> Self {
        // seed runtime options from the execution mode
        let options = RuntimeOptions {
            execution: mode.into(),
            ..RuntimeOptions::default()
        };

        Self::from_runtime_options(platform, &options)
    }

    /// Create a runtime context with an explicit replay header.
    pub fn from_replay_header(
        platform: PlatformContext,
        mode: ExecutionMode,
        header: ReplayHeader,
    ) -> Self {
        // seed runtime options from the execution mode
        let options = RuntimeOptions {
            execution: mode.into(),
            ..RuntimeOptions::default()
        };

        Self::from_runtime_options_and_header(platform, &options, header)
    }

    /// Create a runtime context with explicit runtime options.
    pub fn from_runtime_options(platform: PlatformContext, options: &RuntimeOptions) -> Self {
        // build a replay header from options
        let header = Self::replay_header_from_options(options);

        Self::from_runtime_options_and_header(platform, options, header)
    }

    /// Create a runtime context with explicit runtime options and replay header.
    pub fn from_runtime_options_and_header(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
    ) -> Self {
        // resolve time and random options for the execution mode
        let time_options = Self::resolve_time_options(options);
        let random_options = Self::resolve_random_options(options);

        // build runtime subsystems from options
        let time = Clock::from_options(&time_options);
        let random = Random::from_options(&random_options);
        let execution_mode = ExecutionMode::from(options.execution);
        let replay_payload = if options.execution == WorkspaceExecutionMode::Replay {
            header.replay_payload
        } else {
            Self::resolve_replay_payload(options)
        };

        Self {
            state: Arc::new(RuntimeState {
                platform,
                platform_options: options.platform.clone(),
                time,
                random,
                gc_options: options.gc.clone(),
                resources: ResourceTable::default(),
                replay: ReplayController::new(execution_mode, replay_payload, header),
                errors: RuntimeErrorStore::default(),
            }),
        }
    }

    /// Return the platform context.
    pub fn platform(&self) -> &PlatformContext {
        &self.state.platform
    }

    /// Return the runtime platform options.
    pub fn platform_options(&self) -> &PlatformOptions {
        &self.state.platform_options
    }

    /// Return the runtime windows options.
    pub fn windows(&self) -> &PlatformWindowsOptions {
        &self.state.platform_options.windows
    }

    /// Return the runtime clock.
    pub fn time(&self) -> &Clock {
        &self.state.time
    }

    /// Return the runtime random source.
    pub fn random(&self) -> &Random {
        &self.state.random
    }

    /// Return the runtime GC options.
    pub fn gc_options(&self) -> &GcOptions {
        &self.state.gc_options
    }

    /// Return the resource table.
    pub fn resources(&self) -> &ResourceTable {
        &self.state.resources
    }

    /// Return the replay log.
    pub fn replay(&self) -> &ReplayController {
        &self.state.replay
    }

    /// Return the error store.
    pub fn errors(&self) -> &RuntimeErrorStore {
        &self.state.errors
    }

    /// Return a raw pointer to the runtime state for internal use.
    pub(crate) fn state_ptr(&self) -> *const RuntimeState {
        Arc::as_ptr(&self.state)
    }

    fn resolve_time_options(options: &RuntimeOptions) -> TimeOptions {
        // start from the configured time options
        let mut time_options = options.time.clone();

        // force virtual time during replay
        if options.execution == WorkspaceExecutionMode::Replay {
            time_options.mode = TimeMode::Virtual;
        }

        time_options
    }

    fn resolve_random_options(options: &RuntimeOptions) -> RandomOptions {
        // start from the configured random options
        let mut random_options = options.random.clone();

        // force deterministic randomness during replay
        if options.execution == WorkspaceExecutionMode::Replay {
            random_options.mode = RandomMode::Deterministic;
        }

        random_options
    }

    fn resolve_replay_payload(options: &RuntimeOptions) -> ReplayPayload {
        match options.replay_log.payload {
            ReplayPayloadMode::ResultsOnly => ReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => ReplayPayload::ArgumentsAndResults,
        }
    }

    fn replay_header_from_options(options: &RuntimeOptions) -> ReplayHeader {
        // start from the default header
        let replay_payload = Self::resolve_replay_payload(options);
        let mut header = ReplayHeader {
            execution_mode: ExecutionMode::from(options.execution),
            replay_payload,
            ..ReplayHeader::default()
        };

        // apply replay log chunk sizing
        Self::apply_replay_log_options(&options.replay_log, &mut header);

        header
    }

    fn apply_replay_log_options(options: &ReplayLogOptions, header: &mut ReplayHeader) {
        // update chunk sizing from runtime options
        if let Some(chunk_size_mb) = options.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                header.max_chunk_bytes = chunk_bytes;
            }
        }
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new(PlatformContext::new(Vec::new()))
    }
}

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct RuntimeCallContext {
    /// Runtime state for platform bindings.
    runtime: *const RuntimeState,
    /// Scheduler for task queues and timers.
    scheduler: *const Scheduler,
    /// Binding policy for external calls.
    policy: BindingPolicy,
    /// Engine kind for this binding call.
    engine: PolicyEngine,
    /// Execution metadata for the current call.
    execution: ExecutionContext,
}

impl RuntimeCallContext {
    /// Create a runtime call context for TLS.
    pub fn new(runtime: &RuntimeContext, scheduler: &Scheduler, policy: BindingPolicy) -> Self {
        Self {
            runtime: Arc::as_ptr(&runtime.state),
            scheduler,
            policy,
            engine: PolicyEngine::Native,
            execution: current_execution_context(),
        }
    }

    /// Create a runtime call context from raw pointers.
    pub(crate) fn from_raw(
        runtime: *const RuntimeState,
        scheduler: *const Scheduler,
        policy: BindingPolicy,
        engine: PolicyEngine,
    ) -> Self {
        Self {
            runtime,
            scheduler,
            policy,
            engine,
            execution: current_execution_context(),
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
    pub fn replay(&self) -> &ReplayController {
        &self.runtime().replay
    }

    /// Return the current execution context.
    pub const fn execution(&self) -> ExecutionContext {
        self.execution
    }

    /// Return the current task identifier.
    pub const fn task_id(&self) -> Option<TaskId> {
        self.execution.task_id()
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(&self) -> Option<MicrotaskId> {
        self.execution.microtask_id()
    }

    /// Return the current random stream identifier.
    pub const fn random_stream_id(&self) -> RandomStreamId {
        self.execution.random_stream_id()
    }

    /// Return the engine kind for this call context.
    pub const fn engine(&self) -> PolicyEngine {
        self.engine
    }

    /// Clear call-local storage for native bindings.
    pub fn clear_strings(&self) {
        RUNTIME_CALL_STRINGS.with(|store| store.clear());
        RUNTIME_CALL_VALUES.with(|store| store.clear());
    }

    /// Store a string for the duration of the current call.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        RUNTIME_CALL_STRINGS.with(|store| store.store(value))
    }

    /// Store an optional string for the duration of the current call.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        RUNTIME_CALL_STRINGS.with(|store| store.store_option(value))
    }

    /// Store a slice for the duration of the current call.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        RUNTIME_CALL_VALUES.with(|store| store.store_slice(values))
    }

    /// Store an array for the duration of the current call.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        RUNTIME_CALL_VALUES.with(|store| store.store_array(values))
    }

    /// Store a string slice for the duration of the current call.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        RUNTIME_CALL_VALUES.with(|store| store.store_string_slice(values))
    }

    /// Validate the policy against a binding descriptor.
    #[inline]
    pub fn check_policy(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        self.policy.check_for_engine(spec, Some(self.engine))
    }
}

/// Guard that restores the previous TLS runtime call context.
#[derive(Debug)]
pub struct RuntimeCallGuard {
    /// Previous TLS context pointer.
    previous: *const RuntimeCallContext,
}

/// Guard that restores the previous execution context.
#[derive(Debug)]
pub struct ExecutionContextGuard {
    /// Previous execution context.
    previous: ExecutionContext,
}

impl Drop for ExecutionContextGuard {
    /// Restore the previous execution context.
    fn drop(&mut self) {
        RUNTIME_EXECUTION_CONTEXT.with(|slot| slot.set(self.previous));
    }
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
    // swap in the new TLS context and capture the previous one
    let previous = RUNTIME_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const RuntimeCallContext);
        previous
    });

    RuntimeCallGuard { previous }
}

/// Enter an execution context for runtime scheduling.
#[inline]
pub fn enter_execution_context(context: ExecutionContext) -> ExecutionContextGuard {
    let previous = RUNTIME_EXECUTION_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context);
        previous
    });

    ExecutionContextGuard { previous }
}

fn current_execution_context() -> ExecutionContext {
    RUNTIME_EXECUTION_CONTEXT.with(|slot| slot.get())
}

/// Access the current runtime call context for native bindings.
#[inline]
pub fn with_runtime_call_context<T>(
    f: impl FnOnce(&RuntimeCallContext) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    // read the TLS context pointer
    let context = RUNTIME_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::RuntimeContextMissing.boxed());
    }

    // safety: pointer is set by enter_runtime_call_context
    let context = unsafe { &*context };

    // clear call-local string storage
    context.clear_strings();
    f(context)
}
