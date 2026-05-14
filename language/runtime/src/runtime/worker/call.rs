use std::time::Duration;

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::binding::{
    BindingAccess, BindingAffinity, BindingDescriptor, BindingEngine, BindingReplayPayload,
    BindingRoute, RuntimeAccess,
};
use crate::host::{HostError, HostSession, core as host_core};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{EventLoop, MicrotaskId, TaskId};
use crate::simulation::Simulation;
use crate::world::WorldState;
use crate::world::policy::BindingDecision;
use crate::world::scenario::{Hooks, ScenarioCallId};
use crate::world::trace::{EntropySubject, Trace};

use super::{ExecutionContext, RunnableScope, Worker, binding_affinity_name};
use destack_workspace::{ClockSource, RuntimeDiagnosticLevel};

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct BindingCallContext {
    /// Worker state for host bindings.
    pub(crate) worker: *mut Worker,
    /// Event loop for task queues and timers.
    pub(crate) event_loop: *const EventLoop,
    /// Host event boundary for callbacks.
    pub(crate) host: *const HostSession,
    /// Shared world for replay, time, random, and policy.
    pub(crate) world: *mut WorldState,
    /// Engine kind for this binding call.
    pub(crate) engine: BindingEngine,
    /// Currently running task or microtask.
    pub(crate) scope: RunnableScope,
    /// Execution-affinity context for the current call.
    pub(crate) execution_context: ExecutionContext,
}

/// Scope guard that runs after-binding hooks when one binding call completes.
#[derive(Debug)]
pub struct BindingHookGuard<'call> {
    /// Binding call context for hook routing.
    context: &'call BindingCallContext,
    /// Binding descriptor for hook routing.
    spec: BindingDescriptor,
    /// Binding call identifier for before and after correlation.
    call_id: ScenarioCallId,
}

impl Drop for BindingHookGuard<'_> {
    /// Run post-call hooks for this binding call scope.
    fn drop(&mut self) {
        self.context.on_after_binding(self.spec, self.call_id);
    }
}

#[allow(clippy::mut_from_ref)]
impl BindingCallContext {
    /// Borrow the worker state.
    #[inline]
    pub fn worker(&self) -> &Worker {
        unsafe { &*self.worker }
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
    pub fn host(&self) -> &HostSession {
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

    /// Advance host and runtime wait progress for one blocked binding path.
    pub(crate) fn advance_wait_progress(&self) -> RuntimeResult<()> {
        self.host().advance_ingress()
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

            let now = host_core::monotonic_now_ns();

            // stop once the timeout budget is exhausted
            if now >= deadline_ns {
                return Err(host_core::io_would_block(operation, timeout_message));
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
        self.world().clock().source() == ClockSource::Virtual
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
        if execution_context_satisfies(self.execution_context(), spec.affinity()) {
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

    /// Run pre-call policy checks and return one binding decision.
    #[inline]
    fn preflight_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingDecision> {
        // reject execution-affinity mismatches before policy and hooks
        self.ensure_binding_affinity_allowed(spec)?;

        // access and policy
        let access = self.access();
        access.ensure_allowed(spec)?;
        let decision = self.decide_binding(spec)?;
        self.ensure_binding_access_allowed(spec, &decision)?;

        // service runtime-owned host ingress before host bindings execute
        if decision.route == BindingRoute::Host {
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
    pub fn on_before_binding_resolve_route(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<(BindingRoute, BindingHookGuard<'_>)> {
        let decision = self.preflight_binding_call(spec)?;
        let route = decision.route;

        // reject unavailable host bindings before entering the call
        if route == BindingRoute::Host && !spec.supports_current_target() {
            return Err(RuntimeError::from(HostError::not_supported(spec.name)).boxed());
        }

        let call_id = self
            .hooks()
            .on_before_binding(self.world(), spec, Some(self.engine))?;

        let hook_guard = BindingHookGuard {
            context: self,
            spec,
            call_id,
        };
        Ok((route, hook_guard))
    }

    /// Run post-call hooks for one binding descriptor.
    #[inline]
    fn on_after_binding(&self, spec: BindingDescriptor, call_id: ScenarioCallId) {
        self.hooks()
            .on_after_binding(self.world(), spec, Some(self.engine), call_id);
    }

    /// Resolve the binding route for this call context.
    #[inline]
    pub fn resolve_route(&self, spec: BindingDescriptor) -> RuntimeResult<BindingRoute> {
        let decision = self.decide_binding(spec)?;

        Ok(decision.route)
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let access = self.access();
        let requested = access.default_replay_payload();
        self.trace().payload_policy_for_requested(spec, requested)
    }

    /// Resolve one binding policy decision for this call context.
    #[inline]
    fn decide_binding(&self, spec: BindingDescriptor) -> RuntimeResult<BindingDecision> {
        self.world().decide_binding(
            self.hooks().execution_mode(),
            self.worker().runtime_id,
            self.worker().id,
            spec,
            Some(self.engine),
        )
    }
}

/// Return whether one execution context satisfies one binding affinity requirement.
pub(crate) const fn execution_context_satisfies(
    execution_context: ExecutionContext,
    affinity: BindingAffinity,
) -> bool {
    match affinity {
        BindingAffinity::None => true,
        BindingAffinity::Worker => true,
        BindingAffinity::Main => execution_context.is_process_main,
    }
}
