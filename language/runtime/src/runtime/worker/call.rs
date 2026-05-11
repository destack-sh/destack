use std::time::Duration;

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::Session;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::binding::{
    BindingAccess, BindingAffinity, BindingDescriptor, BindingEngine, BindingReplayPayload,
    RuntimeAccess, RuntimeWorld,
};
use crate::runtime::policy::BindingDecision;
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{EventLoop, MicrotaskId, TaskId};
use crate::runtime::trace::{EntropySubject, Trace};
use crate::runtime::world::WorldState;
use crate::runtime::{Hooks, PolicyCallId};
use crate::simulation::Simulation;

use super::{
    ExecutionContext, ExecutionContextId, NativeCallBuilder, RunnableScope, Worker,
    WorkerCallbackControl, WorkerCallbackHandle, binding_affinity_name, current_runnable_scope,
    current_worker_context, with_native_call_arena,
};
use destack_workspace::{RuntimeDiagnosticLevel, TimeMode};

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct BindingCallContext {
    /// Worker state for platform bindings.
    worker: *mut Worker,
    /// Event loop for task queues and timers.
    event_loop: *const EventLoop,
    /// Host state for platform callbacks.
    host: *const Session,
    /// Shared world for replay, time, random, and policy.
    world: *mut WorldState,
    /// Engine kind for this binding call.
    engine: BindingEngine,
    /// Currently running task or microtask.
    scope: RunnableScope,
    /// Execution-affinity context for the current call.
    execution_context: ExecutionContext,
}

/// Scope guard that runs after-binding hooks when one binding call completes.
#[derive(Debug)]
pub struct BindingHookGuard<'call> {
    /// Binding call context for hook routing.
    context: &'call BindingCallContext,
    /// Binding descriptor for hook routing.
    spec: BindingDescriptor,
    /// Binding call identifier for before and after correlation.
    call_id: PolicyCallId,
}

impl Drop for BindingHookGuard<'_> {
    /// Run post-call hooks for this binding call scope.
    fn drop(&mut self) {
        self.context.on_after_binding(self.spec, self.call_id);
    }
}

impl BindingCallContext {
    /// Create a binding call context from raw pointers.
    pub(crate) fn from_raw(
        worker: *mut Worker,
        event_loop: *const EventLoop,
        host: *const Session,
        world: *mut WorldState,
        engine: BindingEngine,
    ) -> Self {
        let event_loop = unsafe { &*event_loop };
        let host = unsafe { &*host };
        let execution_context = event_loop.execution_context(host.is_process_main_context());

        Self {
            worker,
            event_loop: event_loop as *const EventLoop,
            host: host as *const Session,
            world,
            engine,
            scope: current_runnable_scope(),
            execution_context,
        }
    }

    /// Create one VM binding call context from the current-worker execution scope.
    pub(crate) fn from_current_worker_for_vm() -> RuntimeResult<Self> {
        Self::from_current_worker(BindingEngine::Vm)
    }

    /// Create one native binding call context from the current-worker execution scope.
    pub(crate) fn from_current_worker_for_native() -> RuntimeResult<Self> {
        Self::from_current_worker(BindingEngine::Native)
    }

    /// Create one binding call context from the current-worker execution scope.
    fn from_current_worker(engine: BindingEngine) -> RuntimeResult<Self> {
        let context = current_worker_context()
            .ok_or_else(|| RuntimeError::BindingCallContextMissing.boxed())?;
        Ok(Self::from_raw(
            context.worker,
            context.event_loop,
            context.host,
            context.world,
            engine,
        )
        .with_execution_context(ExecutionContext::new(
            context.execution_context_id,
            context.is_process_main,
        )))
    }

    /// Override the execution context for one raw binding call context.
    fn with_execution_context(mut self, execution_context: ExecutionContext) -> Self {
        self.execution_context = execution_context;
        self
    }

    /// Borrow the worker state.
    #[inline]
    pub fn worker(&self) -> &Worker {
        unsafe { &*self.worker }
    }

    /// Borrow the worker mutably.
    #[inline]
    pub(crate) fn worker_mut(&self) -> &mut Worker {
        unsafe { &mut *self.worker }
    }

    /// Borrow the runtime diagnostics store.
    #[inline]
    pub fn diagnostics(&self) -> &DiagnosticStore {
        self.worker().diagnostics.as_ref()
    }

    /// Record one runtime diagnostic event.
    pub fn record_diagnostic(
        &self,
        level: RuntimeDiagnosticLevel,
        module: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.diagnostics()
            .record(level, module, operation, message.into(), os_code);
    }

    /// Record one runtime warning diagnostic event.
    pub fn warn(
        &self,
        module: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.record_diagnostic(
            RuntimeDiagnosticLevel::Warn,
            module,
            operation,
            message,
            os_code,
        );
    }

    /// Borrow the event loop.
    #[inline]
    pub fn event_loop(&self) -> &EventLoop {
        unsafe { &*self.event_loop }
    }

    /// Borrow immutable process arguments.
    #[inline]
    pub fn process_args(&self) -> &[String] {
        self.worker().process_args()
    }

    /// Borrow the trace state.
    #[inline]
    pub fn trace(&self) -> &Trace {
        self.world().trace()
    }

    /// Borrow the binding access for this worker.
    #[inline]
    fn access(&self) -> parking_lot::RwLockReadGuard<'_, BindingAccess> {
        self.worker().bindings.access().read()
    }

    /// Build one entropy replay subject for the current call and one binding.
    pub fn entropy_subject(&self, spec: BindingDescriptor) -> EntropySubject {
        EntropySubject {
            runtime_id: self.worker().runtime_id,
            worker_id: self.worker().id,
            binding_id: spec.id,
            engine: Some(self.engine()),
            task_id: self.task_id(),
            microtask_id: self.microtask_id(),
        }
    }

    /// Borrow the runtime hook state.
    #[inline]
    pub fn hooks(&self) -> &Hooks {
        self.worker().hooks.as_ref()
    }

    /// Borrow the runtime host state.
    #[inline]
    pub fn host(&self) -> &Session {
        unsafe { &*self.host }
    }

    /// Borrow the shared runtime world.
    #[inline]
    pub(crate) fn world(&self) -> &mut WorldState {
        unsafe { &mut *self.world }
    }

    /// Borrow one read guard for the simulation state.
    #[inline]
    pub fn simulation(&self) -> &Simulation {
        self.world().simulation()
    }

    /// Return the currently running task or microtask.
    pub const fn scope(&self) -> RunnableScope {
        self.scope
    }

    /// Return the execution-affinity context for this call.
    pub const fn execution_context(&self) -> ExecutionContext {
        self.execution_context
    }

    /// Return the current execution context identifier for this call.
    pub const fn execution_context_id(&self) -> ExecutionContextId {
        self.execution_context.id
    }

    /// Advance host and runtime wait progress for one blocked binding path.
    pub(crate) fn advance_wait_progress(&self) -> RuntimeResult<()> {
        // host owned ingress
        self.host().advance_ingress()?;

        // worker local callbacks
        self.worker_mut().service_worker_callbacks(self)
    }

    /// Schedule one worker-local callback.
    pub(crate) fn schedule_worker_callback(
        &self,
        delay_ns: u64,
        interval_ns: Option<u64>,
        callback: impl FnMut(&BindingCallContext) -> RuntimeResult<WorkerCallbackControl>
        + Send
        + 'static,
    ) -> RuntimeResult<WorkerCallbackHandle> {
        self.worker_mut()
            .schedule_worker_callback(self, delay_ns, interval_ns, callback)
    }

    /// Cancel one worker-local callback.
    pub(crate) fn cancel_worker_callback(&self, handle: WorkerCallbackHandle) -> RuntimeResult<()> {
        self.worker_mut().cancel_worker_callback(self, handle)
    }

    /// Wait for one binding result while runtime-owned host ingress makes progress.
    pub fn wait_for_binding_result<T>(
        &self,
        operation: &'static str,
        timeout_message: &'static str,
        deadline_ns: u64,
        mut try_take: impl FnMut() -> RuntimeResult<Option<T>>,
        mut wait_once: impl FnMut(Duration),
    ) -> RuntimeResult<T> {
        // keep polling already-published state before servicing host ingress
        loop {
            if let Some(result) = try_take()? {
                return Ok(result);
            }

            // let the runtime and host own progress while the binding waits
            self.advance_wait_progress()?;

            if let Some(result) = try_take()? {
                return Ok(result);
            }

            let now = core_platform::monotonic_now_ns();

            // stop once the timeout budget is exhausted
            if now >= deadline_ns {
                return Err(core_platform::io_would_block(operation, timeout_message));
            }

            // wait until the next backend publication or the overall deadline
            let remaining = deadline_ns.saturating_sub(now);
            let duration = Duration::from_nanos(remaining);
            wait_once(duration);
        }
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
    pub fn random_stream_id(&self) -> RandomStreamId {
        // resolve runtime and worker scoped stream selection policy
        let worker = self.worker();
        let is_per_runnable = worker.options.random_options().per_runnable;
        let task_id = if is_per_runnable {
            self.scope.task_id().map(TaskId::get)
        } else {
            None
        };
        let microtask_id = if is_per_runnable {
            self.scope.microtask_id().map(MicrotaskId::get)
        } else {
            None
        };

        // resolve one stable world scoped stream id
        self.world().random().scoped_stream_id(
            worker.runtime_id.0,
            worker.id.0,
            task_id,
            microtask_id,
        )
    }

    /// Return true when the world clock runs in virtual mode.
    #[inline]
    pub fn is_virtual_clock(&self) -> bool {
        self.world().time_mode() == TimeMode::Virtual
    }

    /// Return one runtime-backed wall clock sample.
    #[inline]
    pub fn wall_nanos(&self) -> u64 {
        self.world().wall_nanos()
    }

    /// Return one runtime-backed monotonic clock sample.
    #[inline]
    pub fn mono_nanos(&self) -> u64 {
        self.world().mono_nanos()
    }

    /// Notify policy hooks about one clock-read operation.
    #[inline]
    pub fn on_time_read(&self) {
        self.hooks().on_time_read(self.world(), Some(self.engine()));
    }

    /// Notify policy hooks about one random-read operation.
    #[inline]
    pub fn on_random_read(&self) {
        self.hooks()
            .on_random_read(self.world(), Some(self.engine()));
    }

    /// Sleep one runtime-backed duration.
    #[inline]
    pub fn sleep_nanos(&self, duration: u64) {
        self.world().clock().host_sleep_nanos(duration);
    }

    /// Sleep until one runtime-backed wall deadline.
    #[inline]
    pub fn sleep_until_wall_nanos(&self, deadline: u64) {
        self.world().clock().host_sleep_until_nanos(deadline);
    }

    /// Sleep until one runtime-backed monotonic deadline.
    #[inline]
    pub fn sleep_until_mono_nanos(&self, deadline: u64) {
        let now = self.mono_nanos();
        if deadline <= now {
            return;
        }

        let delta = deadline.saturating_sub(now);
        self.sleep_nanos(delta);
    }

    /// Return the engine kind for this call context.
    pub const fn engine(&self) -> BindingEngine {
        self.engine
    }

    /// Clear call-local storage for native bindings.
    pub fn clear_values(&self) {
        with_native_call_arena(|arena| arena.clear());
    }

    /// Store a string for the duration of the current call.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        with_native_call_arena(|arena| arena.store_string(value))
    }

    /// Store one owned string for the duration of the current call.
    pub fn store_string_owned(&self, value: String) -> NativeStringRef {
        with_native_call_arena(|arena| arena.store_string_owned(value))
    }

    /// Store an optional string for the duration of the current call.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        with_native_call_arena(|arena| arena.store_string_option(value))
    }

    /// Store a slice for the duration of the current call.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        with_native_call_arena(|arena| arena.store_slice(values))
    }

    /// Build and store one slice for the duration of the current call.
    pub fn store_slice_with<T: 'static>(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut NativeCallBuilder<T>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeSlice<T>> {
        with_native_call_arena(|arena| arena.store_slice_with(capacity, fill))
    }

    /// Copy a slice for the duration of the current call.
    pub fn store_slice_copy<T: Copy + 'static>(&self, values: &[T]) -> NativeSlice<T> {
        with_native_call_arena(|arena| arena.store_slice_copy(values))
    }

    /// Store one zeroed byte slice for the duration of the current call.
    pub fn store_zeroed_byte_slice(&self, len: usize) -> NativeSlice<u8> {
        with_native_call_arena(|arena| arena.store_zeroed_byte_slice(len))
    }

    /// Store an array for the duration of the current call.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        with_native_call_arena(|arena| arena.store_array(values))
    }

    /// Build and store one array for the duration of the current call.
    pub fn store_array_with<T: 'static>(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut NativeCallBuilder<T>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeArray<T>> {
        with_native_call_arena(|arena| arena.store_array_with(capacity, fill))
    }

    /// Copy an array for the duration of the current call.
    pub fn store_array_copy<T: Copy + 'static>(&self, values: &[T]) -> NativeArray<T> {
        with_native_call_arena(|arena| arena.store_array_copy(values))
    }

    /// Store one zeroed byte array for the duration of the current call.
    pub fn store_zeroed_byte_array(&self, len: usize) -> NativeArray<u8> {
        with_native_call_arena(|arena| arena.store_zeroed_byte_array(len))
    }

    /// Store a string slice for the duration of the current call.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        with_native_call_arena(|arena| arena.store_string_slice(values))
    }

    /// Build and store one string slice for the duration of the current call.
    pub fn store_string_slice_with(
        &self,
        capacity: usize,
        fill: impl FnOnce(&mut NativeCallBuilder<NativeStringRef>) -> RuntimeResult<()>,
    ) -> RuntimeResult<NativeStringSlice> {
        with_native_call_arena(|arena| arena.store_string_slice_with(capacity, fill))
    }

    /// Return one policy-violation error for one binding descriptor.
    fn policy_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::PolicyViolation {
            name: spec.name.to_string(),
        }
    }

    /// Return one affinity-violation error for one binding descriptor.
    fn affinity_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::AffinityViolation {
            name: spec.name.to_string(),
            affinity: binding_affinity_name(spec.affinity()).to_string(),
        }
    }

    /// Ensure the current execution context satisfies one binding affinity.
    fn ensure_binding_affinity_allowed(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        if execution_context_satisfies(
            self.execution_context(),
            self.event_loop().execution_context_id(),
            spec.affinity(),
        ) {
            return Ok(());
        }

        Err(self.affinity_violation_error(spec).boxed())
    }

    /// Ensure world access routing allows this binding call.
    fn ensure_binding_access_allowed(
        &self,
        spec: BindingDescriptor,
        decision: &BindingDecision,
    ) -> RuntimeResult<()> {
        if decision.access == RuntimeAccess::Deny {
            return Err(self.policy_violation_error(spec).boxed());
        }

        Ok(())
    }

    /// Run pre-call policy checks and return one binding decision snapshot.
    #[inline]
    fn preflight_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingDecision> {
        // reject execution-affinity mismatches before policy and hooks
        self.ensure_binding_affinity_allowed(spec)?;

        // access and policy
        let access = self.access();
        access.ensure_allowed(spec)?;
        let decision = self.binding_decision(spec, access.default_replay_payload())?;
        self.ensure_binding_access_allowed(spec, &decision)?;

        // service runtime-owned host ingress before host bindings execute
        if decision.world == RuntimeWorld::Host {
            self.advance_wait_progress()?;
        }

        Ok(decision)
    }

    /// Run pre-call binding policy and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingHookGuard<'_>> {
        self.preflight_binding_call(spec)?;
        let call_id = self
            .hooks()
            .on_before_binding(self.world(), spec, Some(self.engine))?;

        Ok(BindingHookGuard {
            context: self,
            spec,
            call_id,
        })
    }

    /// Run pre-call policy, resolve world, and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding_resolve_world(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<(RuntimeWorld, BindingHookGuard<'_>)> {
        let decision = self.preflight_binding_call(spec)?;
        let world = decision.world;

        // reject unavailable host bindings before entering the call
        if world == RuntimeWorld::Host && !spec.supports_current_target() {
            return Err(RuntimeError::from(PlatformError::not_supported(spec.name)).boxed());
        }

        let call_id = self
            .hooks()
            .on_before_binding(self.world(), spec, Some(self.engine))?;

        let hook_guard = BindingHookGuard {
            context: self,
            spec,
            call_id,
        };
        Ok((world, hook_guard))
    }

    /// Run post-call hooks for one binding descriptor.
    #[inline]
    fn on_after_binding(&self, spec: BindingDescriptor, call_id: PolicyCallId) {
        self.hooks()
            .on_after_binding(self.world(), spec, Some(self.engine), call_id);
    }

    /// Resolve the binding world for this call context.
    #[inline]
    pub fn resolve_world(&self, spec: BindingDescriptor) -> RuntimeResult<RuntimeWorld> {
        let access = self.access();
        let decision = self.binding_decision(spec, access.default_replay_payload())?;

        Ok(decision.world)
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let access = self.access();
        let decision = self.binding_decision(spec, access.default_replay_payload())?;

        let requested = decision.replay_payload;
        self.trace().payload_policy_for_requested(spec, requested)
    }

    /// Resolve one binding policy decision for this call context.
    #[inline]
    fn binding_decision(
        &self,
        spec: BindingDescriptor,
        default_replay_payload: BindingReplayPayload,
    ) -> RuntimeResult<BindingDecision> {
        self.world().resolve_binding(
            self.hooks().execution_mode(),
            self.worker().runtime_id,
            self.worker().id,
            spec,
            Some(self.engine),
            RuntimeAccess::Allow,
            RuntimeWorld::Host,
            default_replay_payload,
        )
    }
}

/// Return whether one execution context satisfies one binding affinity requirement.
pub(crate) const fn execution_context_satisfies(
    execution_context: ExecutionContext,
    worker_context_id: ExecutionContextId,
    affinity: BindingAffinity,
) -> bool {
    match affinity {
        BindingAffinity::None => true,
        BindingAffinity::Worker => execution_context.id.0 == worker_context_id.0,
        BindingAffinity::Main => execution_context.is_process_main,
    }
}
